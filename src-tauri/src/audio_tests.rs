use super::*;
use std::fs;

#[test]
fn test_stop_latency_under_500ms() {
    let latency_ms = voice_stop(None).expect("voice_stop should succeed");
    assert!(
        latency_ms < 500,
        "voice_stop latency must be <500ms, got {}ms",
        latency_ms
    );
}

#[test]
fn test_sentence_wav_generation() {
    let wav_path = generate_sentence_wav("Unit test audio sentence.", None)
        .expect("Sentence WAV generation should succeed");
    assert!(wav_path.exists(), "Generated WAV file must exist on disk");
    let metadata = fs::metadata(&wav_path).expect("Metadata read should succeed");
    assert!(
        metadata.len() > 44,
        "WAV file must contain audio frames beyond standard 44-byte RIFF header"
    );
}

#[test]
fn test_hound_wav_spec_encoding() {
    let tmp_path = get_app_data_audio_dir().join("test_hound_spec.wav");
    let spec = WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = WavWriter::create(&tmp_path, spec).expect("WavWriter create failed");
    for i in 0..1600 {
        let sample = ((i % 100) * 100) as i16;
        writer.write_sample(sample).expect("Sample write failed");
    }
    writer.finalize().expect("Finalize failed");

    assert!(tmp_path.exists());
    let reader = hound::WavReader::open(&tmp_path).expect("WavReader open failed");
    assert_eq!(reader.spec().channels, 1);
    assert_eq!(reader.spec().sample_rate, 16000);
    assert_eq!(reader.spec().bits_per_sample, 16);
    assert_eq!(reader.len(), 1600);

    let _ = fs::remove_file(&tmp_path);
}

#[test]
fn test_voice_record_lifecycle() {
    let _ = voice_record_start();
    std::thread::sleep(Duration::from_millis(50));
    let res = voice_record_stop().expect("voice_record_stop should succeed");
    assert!(!res.wav_path.is_empty());
    let path = PathBuf::from(&res.wav_path);
    assert!(path.exists());
    let _ = fs::remove_file(&path);
}
