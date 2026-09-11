use std::path::Path;

use super::fixture_server;

#[allow(dead_code)]
pub fn fixtures_hls_dir() -> std::path::PathBuf {
    fixture_server::fixtures_dir().join("hls")
}

#[allow(dead_code)]
pub fn build_multi_segment_hls_fixture(dir: &Path, segment_count: usize) {
    let seg = std::fs::read(fixtures_hls_dir().join("segments/seg0.ts")).unwrap();
    std::fs::create_dir_all(dir.join("segments")).unwrap();
    let mut playlist = String::from(
        "#EXTM3U\n#EXT-X-VERSION:3\n#EXT-X-TARGETDURATION:10\n#EXT-X-MEDIA-SEQUENCE:0\n",
    );
    for index in 0..segment_count {
        std::fs::write(dir.join(format!("segments/seg{index}.ts")), &seg).unwrap();
        playlist.push_str(&format!("#EXTINF:1.5,\nsegments/seg{index}.ts\n"));
    }
    playlist.push_str("#EXT-X-ENDLIST\n");
    std::fs::write(dir.join("many.m3u8"), playlist).unwrap();
}
