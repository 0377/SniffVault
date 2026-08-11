mod json_api {
    use video_sniffing_engine::EngineError;
    use video_sniffing_engine_ffi::json_api::{err_json, map_engine_error, ok_json};

    #[test]
    fn ok_json_wraps_data() {
        let json = ok_json(serde_json::json!({"count": 42}));
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["ok"], true);
        assert_eq!(parsed["data"]["count"], 42);
    }

    #[test]
    fn err_json_maps_invalid_arg() {
        let err = EngineError::InvalidArg("bad input".into());
        let json = err_json(err);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["ok"], false);
        assert_eq!(parsed["error"]["kind"], "invalid_arg");
        assert_eq!(parsed["error"]["message"], "bad input");
    }

    #[test]
    fn map_not_found() {
        let err = map_engine_error(EngineError::NotFound("item-1".into()));
        assert_eq!(err.kind, "not_found");
        assert_eq!(err.message, "item-1");
    }
}
