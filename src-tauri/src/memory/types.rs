use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct PersonExtract {
    pub name: String,
    #[serde(default)]
    pub relation: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReminderExtract {
    pub content: String,
    #[serde(default)]
    pub due: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PreferenceExtract {
    pub topic: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct EpisodeExtract {
    pub summary: String,
    #[serde(default)]
    pub people: Vec<String>,
    #[serde(default)]
    pub place: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct MemoryExtraction {
    #[serde(default)]
    pub people: Vec<PersonExtract>,
    #[serde(default)]
    pub reminders: Vec<ReminderExtract>,
    #[serde(default)]
    pub preferences: Vec<PreferenceExtract>,
    #[serde(default)]
    pub episode: Option<EpisodeExtract>,
}
