#[derive(Debug, Default)]
pub struct LuciMemory {
    pub mood: String,
    pub facts: Vec<String>,
    pub plans: Vec<String>,
}

impl LuciMemory {
    pub fn remember(&mut self, content: &str) {
        self.facts.push(content.trim().to_string());
    }

    pub fn add_plan(&mut self, title: &str) {
        self.plans.push(title.trim().to_string());
    }
}
