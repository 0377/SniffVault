use crate::download::hls::playlist::{resolve_url, KeyTag, MediaPlaylist, SegmentEntry};
use crate::download::http::HttpClient;
use crate::error::EngineError;
use aes::cipher::{block_padding::NoPadding, BlockDecryptMut, KeyIvInit};
use aes::Aes128;
use cbc::Decryptor;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;

type Aes128CbcDec = Decryptor<Aes128>;

/// AES-128-CBC 解密 HLS TS 分片，不使用 PKCS7（密文须 16 字节对齐）。
pub fn decrypt_aes128_ts(
    ciphertext: &[u8],
    key: &[u8; 16],
    iv: &[u8; 16],
) -> Result<Vec<u8>, EngineError> {
    if ciphertext.is_empty() {
        return Ok(Vec::new());
    }
    if !ciphertext.len().is_multiple_of(16) {
        return Err(EngineError::Message(
            "AES-128 TS ciphertext must be 16-byte aligned".into(),
        ));
    }

    let mut buf = ciphertext.to_vec();
    let dec = Aes128CbcDec::new(key.into(), iv.into());
    dec.decrypt_padded_mut::<NoPadding>(&mut buf)
        .map_err(|err| EngineError::Message(format!("AES-128 decrypt failed: {err}")))?;
    Ok(buf)
}

pub fn segment_iv(
    key: &KeyTag,
    media_sequence: u32,
    segment_index: usize,
) -> Result<[u8; 16], EngineError> {
    if let Some(iv_hex) = &key.iv_hex {
        return parse_iv_hex(iv_hex);
    }
    let sequence = media_sequence as u64 + segment_index as u64;
    Ok(iv_from_sequence(sequence))
}

pub fn iv_from_sequence(sequence: u64) -> [u8; 16] {
    let mut iv = [0u8; 16];
    iv[8..16].copy_from_slice(&sequence.to_be_bytes());
    iv
}

fn parse_iv_hex(iv_hex: &str) -> Result<[u8; 16], EngineError> {
    let hex_str = iv_hex.strip_prefix("0x").unwrap_or(iv_hex);
    let bytes = hex::decode(hex_str)
        .map_err(|err| EngineError::InvalidArg(format!("invalid IV hex: {err}")))?;
    if bytes.len() != 16 {
        return Err(EngineError::InvalidArg(format!(
            "IV must be 16 bytes, got {}",
            bytes.len()
        )));
    }
    let mut iv = [0u8; 16];
    iv.copy_from_slice(&bytes);
    Ok(iv)
}

pub async fn fetch_aes128_key(
    http: &HttpClient,
    key_uri: &str,
    playlist_url: &str,
) -> Result<[u8; 16], EngineError> {
    let url = resolve_url(playlist_url, key_uri)?;
    let bytes = http.get_bytes(&url).await?;
    if bytes.len() != 16 {
        return Err(EngineError::Message(format!(
            "AES-128 key must be 16 bytes, got {}",
            bytes.len()
        )));
    }
    let mut key = [0u8; 16];
    key.copy_from_slice(&bytes);
    Ok(key)
}

pub async fn download_segment(
    http: &HttpClient,
    segment: &SegmentEntry,
    dest: &Path,
    key: Option<&[u8; 16]>,
    key_tag: Option<&KeyTag>,
    media_sequence: u32,
    segment_index: usize,
) -> Result<(), EngineError> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).await?;
    }

    let ciphertext = http.get_bytes(&segment.uri).await?;
    let payload = if let (Some(key), Some(tag)) = (key, key_tag) {
        if !tag.method.eq_ignore_ascii_case("AES-128") {
            return Err(EngineError::Message(format!(
                "unsupported encryption method: {}",
                tag.method
            )));
        }
        let iv = segment_iv(tag, media_sequence, segment_index)?;
        decrypt_aes128_ts(&ciphertext, key, &iv)?
    } else {
        ciphertext
    };

    let mut file = fs::File::create(dest).await?;
    file.write_all(&payload).await?;
    file.flush().await?;
    Ok(())
}

pub async fn download_segments(
    http: &HttpClient,
    playlist: &MediaPlaylist,
    playlist_url: &str,
    temp_dir: &Path,
    skip_indices: &[u32],
    existing_paths: &[PathBuf],
) -> Result<Vec<PathBuf>, EngineError> {
    fs::create_dir_all(temp_dir).await?;

    let aes_key = if let Some(tag) = playlist.encryption.as_ref() {
        Some(fetch_aes128_key(http, &tag.uri, playlist_url).await?)
    } else {
        None
    };

    let mut paths = existing_paths.to_vec();
    for (index, segment) in playlist.segments.iter().enumerate() {
        let index_u32 = index as u32;
        if skip_indices.contains(&index_u32) {
            continue;
        }

        let dest = temp_dir.join(format!("seg{index:04}.ts"));
        download_segment(
            http,
            segment,
            &dest,
            aes_key.as_ref(),
            playlist.encryption.as_ref(),
            playlist.media_sequence,
            index,
        )
        .await?;
        paths.push(dest);
    }

    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decrypt_aes128_ts_roundtrip_without_pkcs7() {
        use aes::cipher::{block_padding::NoPadding, BlockEncryptMut, KeyIvInit};
        use cbc::Encryptor;

        type Aes128CbcEnc = Encryptor<Aes128>;

        let key = *b"0123456789abcdef";
        let iv = iv_from_sequence(0);
        let plaintext = b"0123456789abcdef".repeat(4);

        let mut ciphertext = plaintext.clone();
        let enc = Aes128CbcEnc::new(&key.into(), &iv.into());
        enc.encrypt_padded_mut::<NoPadding>(&mut ciphertext, plaintext.len())
            .unwrap();

        let decrypted = decrypt_aes128_ts(&ciphertext, &key, &iv).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn rejects_unaligned_ciphertext() {
        let key = [0u8; 16];
        let iv = [0u8; 16];
        let err = decrypt_aes128_ts(&[1, 2, 3], &key, &iv).unwrap_err();
        assert!(matches!(err, EngineError::Message(_)));
    }

    #[test]
    fn iv_from_sequence_is_big_endian_right_aligned() {
        let iv = iv_from_sequence(42);
        assert_eq!(&iv[..8], &[0u8; 8]);
        assert_eq!(&iv[8..], &42u64.to_be_bytes());
    }
}
