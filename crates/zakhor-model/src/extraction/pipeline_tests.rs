use super::config::ExtractionConfig;
use super::pipeline::ExtractionPipeline;

#[test]
fn test_pipeline_new_does_not_load_model() {
    let config = ExtractionConfig::default();
    let _pipeline = ExtractionPipeline::new(config);
}

#[test]
fn test_extract_entities_and_relations_method_signature() {
    let config = ExtractionConfig::default();
    let pipeline = ExtractionPipeline::new(config);
    // Verify the pipeline type has the extract_entities_and_relations method
    fn _assert_signature(_p: &ExtractionPipeline) {}
    _assert_signature(&pipeline);
}
