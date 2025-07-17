use super::types::Sampler;

pub struct TopPSampler {
    target_provider: String,
    value: f32,
    shift_values: bool,
    shift_amount: f32
}

impl TopPSampler {
    pub fn new(target_provider: &str, value: f32) -> Self {
        Self {
            target_provider: target_provider.to_string(),
            value,
            shift_values: false,
            shift_amount: 0.0
        }
    }

    pub fn set_shift_policy(&mut self, shift_values: bool, shift_amount: f32) -> &mut Self {
        self.shift_values = shift_values;
        self.shift_amount = shift_amount;

        self
    }

    fn get_name(&self) -> String {
        let provider = match self.target_provider.as_str() {
            "openrouter" => "top_p",
            _ => "top_p"
        };

        provider.to_string()
    }

    fn get_min_max(&self) -> (f32, f32) {
        match self.target_provider.as_str() {
            "openrouter" => (0.0, 1.0),
            _ => (0.0, 1.0),  
        }
    }

    fn clamp_value(&self, value: f32) -> f32 {
        let (min, max) = self.get_min_max();
        value.clamp(min, max)
    }
}

impl Sampler for TopPSampler {
    fn name(&self) -> String {
        self.get_name()
    }

    fn value(&self) -> super::types::SamplerValue {
        let value = if self.shift_values {
            self.value + self.shift_amount
        } else {
            self.value
        };

        super::types::SamplerValue::Float(self.clamp_value(value))
    }
}