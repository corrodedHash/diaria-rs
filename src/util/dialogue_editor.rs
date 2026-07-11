use dialoguer::Editor;

#[mockall::automock]
pub trait DialogueEditor {
    fn edit(&self, prompt: &str) -> std::io::Result<String>;
}

pub struct RealDialogueEditor;

impl DialogueEditor for RealDialogueEditor {
    fn edit(&self, _prompt: &str) -> std::io::Result<String> {
        Editor::new()
            .edit("")
            .map_err(std::io::Error::other)
            .map(std::option::Option::unwrap_or_default)
    }
}
