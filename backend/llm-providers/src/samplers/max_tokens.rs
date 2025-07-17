use super::types::Sampler;

pub struct MaxTokensSampler {
    target_provider: String,
    value: i32,
}

impl MaxTokensSampler {
    pub fn new(target_provider: &str, value: i32) -> MaxTokensSampler {
        MaxTokensSampler { target_provider: target_provider.to_string(), value }
    }
}

impl Sampler for MaxTokensSampler {
    fn name(&self) -> String {
        "max_tokens".to_string()
    }

    fn value(&self) -> super::types::SamplerValue {
        self.value.into()
    }
}