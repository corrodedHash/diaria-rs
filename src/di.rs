use crate::commands::{
    CmdAdd, CmdDump, CmdInit, CmdLoad, CmdRead, CmdStats, CmdStatus, CmdSummarize, CmdSync,
};
use crate::entry::key_manager::{DiariaKeyManager, FsKeyManager};
use crate::entry::repository::{DiariaEntryRepository, DiariaFsRepository, DiariaMetaRepository};
use crate::util::dialogue_editor::{DialogueEditor, RealDialogueEditor};
use crate::util::entry_selector::{EntrySelector, RealEntrySelector};
use crate::util::environment::{Environment, SystemEnvironment};
use crate::util::file_loader::{FileLoader, RealFileLoader};
use crate::util::password::{PasswordService, TerminalPasswordService};
use crate::util::stdout_printer::{RealUserOutput, UserOutput};

/// The composition root: the single place that decides which concrete
/// implementation backs each dependency in a production build. Tests wire
/// their own mocks directly through each command's `new`.
pub struct Di;

impl Di {
    fn entry_repo() -> Box<dyn DiariaEntryRepository> {
        Box::new(DiariaFsRepository {})
    }

    fn meta_repo() -> Box<dyn DiariaMetaRepository> {
        Box::new(DiariaFsRepository {})
    }

    fn environment() -> Box<dyn Environment> {
        Box::new(SystemEnvironment)
    }

    fn password() -> Box<dyn PasswordService> {
        Box::new(TerminalPasswordService::new(Self::environment()))
    }

    fn user_output() -> Box<dyn UserOutput> {
        Box::new(RealUserOutput)
    }

    fn file_loader() -> Box<dyn FileLoader> {
        Box::new(RealFileLoader)
    }

    fn dialogue_editor() -> Box<dyn DialogueEditor> {
        Box::new(RealDialogueEditor)
    }

    fn entry_selector() -> Box<dyn EntrySelector> {
        Box::new(RealEntrySelector)
    }

    fn key_manager() -> Box<dyn DiariaKeyManager> {
        Box::new(FsKeyManager::new(Self::meta_repo(), Self::password()))
    }

    pub fn init() -> CmdInit {
        CmdInit::new(Self::meta_repo(), Self::password(), Self::user_output())
    }

    pub fn add() -> CmdAdd {
        CmdAdd::new(
            Self::entry_repo(),
            Self::key_manager(),
            Self::file_loader(),
            Self::dialogue_editor(),
            Self::user_output(),
        )
    }

    pub fn read() -> CmdRead {
        CmdRead::new(
            Self::entry_repo(),
            Self::key_manager(),
            Self::user_output(),
            Self::entry_selector(),
        )
    }

    pub fn stats() -> CmdStats {
        CmdStats::new(Self::entry_repo(), Self::user_output())
    }

    pub fn status() -> CmdStatus {
        CmdStatus::new(Self::meta_repo(), Self::entry_repo(), Self::user_output())
    }

    pub fn load() -> CmdLoad {
        CmdLoad::new(Self::entry_repo(), Self::key_manager(), Self::user_output())
    }

    pub fn dump() -> CmdDump {
        CmdDump::new(Self::entry_repo(), Self::key_manager(), Self::user_output())
    }

    pub fn sync() -> CmdSync {
        CmdSync::new(Self::meta_repo(), Self::user_output())
    }

    pub fn summarize() -> CmdSummarize {
        CmdSummarize::new(Self::entry_repo(), Self::key_manager(), Self::user_output())
    }
}
