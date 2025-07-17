#[derive(Default)]
pub struct SamplerConfig {
    // For the temperature sampler, configuration could include:
    // initial value, whether to shift values, and the shift amount.
    pub temperature: Option<(f32, bool, f32)>,
    // For TopP (or Top PSampler), it might only need an initial value.
    pub top_p: Option<f32>,
    pub max_tokens: Option<i32>
}
