// src/core/trigger/action.rs
//
// Trigger Actions Definition for Tylogger (LogExpert Rust Reimplementation)
// Verified upstream actions: Set Bookmark, Stop Follow Tail, Audio Alert, Plugin Keyword Action.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriggerAction {
    SetBookmark {
        label: Option<String>,
        comment: Option<String>,
    },
    StopFollowTail,
    AudioAlert {
        sound_path: Option<String>,
        use_default_beep: bool,
    },
    DesktopNotification {
        title: String,
        message: String,
    },
    PluginKeywordAction {
        plugin_id: String,
        action_name: String,
    },
    IncrementCounter {
        counter_name: String,
    },
}
