//! A build's dashboards: the layout its author chose to show the build
//! with (which components, which calc result fields), a planner's setting
//! kept with the build. Layout only, never calculated numbers.
//!
//! `kind`, `width`, and `vary` are open text on purpose: the UI adds
//! component kinds without a change here, and an application older than a
//! kind still opens the build and shows what it knows.

/// A build's dashboards and which one is shown.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Dashboards {
    /// The ID of the dashboard shown ([`Dashboard::id`]).
    pub active: String,
    /// The dashboards, in order.
    pub list: Vec<Dashboard>,
}

/// One dashboard: a named list of components.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Dashboard {
    /// The dashboard's ID, unique among the build's dashboards.
    pub id: String,
    /// The name its author gave it.
    pub name: String,
    /// The components, in order.
    pub components: Vec<DashboardComponent>,
}

/// One component of a dashboard: what it shows, never the numbers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DashboardComponent {
    /// The component's ID, unique among the build's dashboards.
    pub id: String,
    /// The component's kind (`stat`, `progression`, `timeline`, ...): open
    /// text, so the UI adds kinds without a change here and an older
    /// application still opens the build.
    pub kind: String,
    /// The component's width (`third`, `half`, `full`, ...): open text, as
    /// `kind`.
    pub width: String,
    /// The calc result field a stat, history, or curve component shows.
    pub field: Option<String>,
    /// The calc result fields a progression component charts, in order.
    pub fields: Vec<String>,
    /// What a scaling curve varies (`gem-level`, `character-level`,
    /// `config-number`): open text, as `kind`.
    pub vary: Option<String>,
    /// The config input a scaling curve varies.
    pub input: Option<String>,
    /// The life a fight timeline deals damage to.
    pub target: Option<u64>,
    /// Whether the component starts a new row, whatever room the row
    /// before it has left.
    pub new_row: bool,
}
