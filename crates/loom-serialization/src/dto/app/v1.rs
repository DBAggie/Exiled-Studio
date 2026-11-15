//! Application API v1 (module docs of [`super`]).
//!
//! Names are kebab-case on the wire, as the documents'. Item and line
//! indices are the domain's: an item's ID within the build, a line's index
//! among the item's source lines (the calc result's `item` sources use the
//! same pair, so a breakdown links to its line).

use serde::{Deserialize, Serialize};

/// The API version this module describes.
pub const VERSION: u32 = 1;

/// Declares DTO types with the shared derives and serde settings.
macro_rules! dto {
    ($($item:item)*) => {
        $(
            #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
            #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
            #[serde(deny_unknown_fields, rename_all = "kebab-case")]
            $item
        )*
    };
}

dto! {
    /// Every top-level type of the API, so one schema names them all. Never
    /// sent.
    pub struct Api {
        /// The snapshot query's answer.
        pub snapshot: Snapshot,
        /// One item.
        pub item: ItemView,
        /// A command.
        pub command: Command,
        /// A command's answer.
        pub outcome: Outcome,
        /// An event.
        pub event: Event,
        /// An error.
        pub error: ApiError,
        /// An item edit's preview.
        pub preview: ItemEditPreview,
        /// The MCP server's state.
        pub mcp: McpStatus,
        /// Recovery snapshots found at start.
        pub recovery: Vec<RecoveryEntry>,
        /// Recent builds.
        pub recent: Vec<RecentEntry>,
        /// The gems the game data offers.
        pub gems: Vec<GemCatalogEntry>,
        /// The loaded data's passive tree.
        pub tree_layout: TreeLayout,
        /// A tree edit's preview.
        pub tree_preview: TreePreview,
        /// The loaded data's passive tree sprites, when its bundle has
        /// them.
        pub tree_sprites: TreeSprites,
        /// Crash records (Phase 5, P-f).
        pub crashes: Vec<CrashRecord>,
        /// The diagnostic package's preview (Phase 5, P-f).
        pub diagnostics: Vec<DiagnosticFile>,
        /// Application updates (Phase 5, P-d).
        pub updates: UpdateStatus,
    }

    /// How the application looks for its own updates (Phase 5, P-d). Every
    /// install waits for the user's approval in each mode.
    #[derive(Copy, Eq, Default)]
    pub enum UpdateMode {
        #[default]
        /// Checks once at start, and when the user asks (the default).
        AutomaticCheck,
        /// Checks only when the user asks.
        Manual,
        /// Never connects: no check, no download.
        StrictOffline,
    }

    /// Application updates: the mode, this build, and the last check's
    /// answer (Phase 5, P-d).
    pub struct UpdateStatus {
        /// The mode.
        pub mode: UpdateMode,
        /// The running version.
        pub current: String,
        /// The package this build came from: `nsis`, `msi`, `app-image`,
        /// `deb`, or `plain` (the portable zip, or a development build).
        pub artifact: String,
        /// Whether this package installs an update itself (`nsis`, `msi`,
        /// `app-image`); the others only link to the release.
        pub installs: bool,
        /// Whether this build can check at all (it carries an update key).
        pub available: bool,
        /// A check runs.
        pub checking: bool,
        /// The last check's newer version, if it found one.
        pub offer: Option<UpdateOffer>,
        /// Whether a check has answered since the application started.
        pub checked: bool,
        /// Why the last check or install failed, if it did.
        pub error: Option<ApiError>,
    }

    /// A newer version the update feed offers (lower or equal versions are
    /// never offered).
    pub struct UpdateOffer {
        /// Its version.
        pub version: String,
        /// Its release notes.
        pub notes: Option<String>,
        /// When it was published (RFC 3339), if the feed says.
        pub date: Option<String>,
        /// Its release page.
        pub url: String,
    }

    /// A panic the application recorded (ADR-014; Phase 5, P-f), redacted.
    pub struct CrashRecord {
        /// Its ID.
        pub id: String,
        /// When, in seconds since 1970 (UTC).
        pub at: u64,
        /// The application's version.
        pub version: String,
        /// The thread's name.
        pub thread: String,
        /// The panic's message.
        pub message: String,
        /// Where it panicked (`file:line:column`).
        pub location: Option<String>,
    }

    /// One file of the diagnostic package, as the user previews it and the
    /// zip holds it.
    pub struct DiagnosticFile {
        /// Its name in the package (`about.json`, `logs/loom.log`, ...).
        pub name: String,
        /// Its size in bytes.
        pub bytes: u64,
        /// Its text, redacted.
        pub text: String,
    }

    /// The application's state at one version. Queries return it whole;
    /// events say when it changed (ADR-004: a missed event version means
    /// query again).
    pub struct Snapshot {
        /// Increases with every change of the build (import, open, edit,
        /// undo, redo); 0 before any build.
        pub version: u64,
        /// The open build, if any.
        pub build: Option<BuildView>,
        /// The calculation of the latest version.
        pub calc: CalcStatus,
        /// Undo and redo.
        pub history: HistoryView,
        /// Whether the build changed since it was last saved or opened.
        pub dirty: bool,
        /// Where the build is saved (a build document), if it is.
        pub path: Option<String>,
        /// The file the build was imported from, if any; never written.
        pub origin: Option<String>,
        /// The game data the application calculates with.
        pub data: DataStatus,
        /// Notices about the open build that are not errors (for example
        /// `APP_RESOLUTION_CHANGED`: the build's stored resolutions differ
        /// from this bundle's, so they stand and rolled values stay
        /// unsettled).
        pub notices: Vec<ApiError>,
    }

    /// The game data.
    pub struct DataStatus {
        /// The bundle folder, if one is set.
        pub bundle: Option<String>,
        /// The bundle's data version, when it loaded.
        pub data_version: Option<DataVersionView>,
        /// The passive tree version the application gives the engine for
        /// this bundle, if known; it names the game's patch (`3_29` is
        /// patch 3.29), which the UI shows (UX-14).
        pub tree_version: Option<String>,
        /// Why the bundle did not load, if it did not (a stable code).
        pub error: Option<ApiError>,
        /// The bundles installed in the application's `bundles/` folder,
        /// newest first (B-1). The UI switches between them by ID with
        /// [`Command::UseBundle`]; paths never reach the webview.
        #[serde(default)]
        pub installed: Vec<InstalledBundle>,
        /// The loaded bundle's ID, when it is an installed one (a folder
        /// picked for development, or `LOOM_BUNDLE_DIR`, has none: it is
        /// unsigned and not installed).
        pub active: Option<String>,
        /// Something to tell the user about the choice of data that is not
        /// an error: `APP_BUNDLE_FALLBACK` when the remembered bundle did
        /// not load and the newest installed one was taken instead.
        pub notice: Option<ApiError>,
        /// The installed bundle the open build was made with, when it is not
        /// the loaded one (ADR-010: never switched silently; the UI offers
        /// it).
        pub pinned: Option<String>,
        /// Installed bundles that do not load (Phase 5, P-e): unsigned or
        /// signed by an unknown key until the user accepts them; for good
        /// when the signature fails, the key is revoked, or files are
        /// damaged.
        #[serde(default)]
        pub quarantined: Vec<QuarantinedBundle>,
    }

    /// An installed bundle that does not load (Phase 5, P-e).
    pub struct QuarantinedBundle {
        /// Its ID: `<game>/<steam build>-s<data schema>`.
        pub id: String,
        /// Its data version.
        pub data_version: DataVersionView,
        /// Why: `unsigned`, `unknown-key`, `bad-signature`, `revoked`,
        /// `damaged`.
        pub reason: String,
        /// Whether the user may accept it ([`Command::AcceptBundle`]):
        /// `unsigned` and `unknown-key` only.
        pub acceptable: bool,
        /// The SHA-256 of its manifest (`bundle.json`), shown to the user,
        /// who accepts exactly it.
        pub manifest_sha256: String,
        /// The signing key's ID, when signed.
        pub key_id: Option<String>,
        /// Its number of files.
        pub files: u32,
        /// Its size in bytes.
        pub bytes: u64,
    }

    /// A bundle installed in the application's `bundles/` folder.
    pub struct InstalledBundle {
        /// Its ID: `<game>/<steam build>-s<data schema>`.
        pub id: String,
        /// Its data version.
        pub data_version: DataVersionView,
        /// The passive tree version the application knows for it, if any:
        /// the game's patch (`3_29` is patch 3.29; UX-14).
        pub tree_version: Option<String>,
        /// The key that signed it, when a trusted one did (Phase 5, P-e).
        #[serde(default)]
        pub signed_by: Option<String>,
        /// Whether the user accepted it from quarantine (unsigned, or an
        /// unknown key).
        #[serde(default)]
        pub accepted: bool,
    }

    /// A data version.
    pub struct DataVersionView {
        /// `poe1` or `poe2`.
        pub game: String,
        /// The bundle's data schema version.
        pub schema_version: u32,
        /// The Steam build it was extracted from, if known.
        pub steam_build: Option<u64>,
    }

    /// The open build.
    pub struct BuildView {
        /// A display name: the file's stem, else "imported build" in the
        /// UI's language (empty here).
        pub name: String,
        /// `poe1` or `poe2`.
        pub game: String,
        /// The class, if set.
        pub class: Option<String>,
        /// The ascendancy, if set.
        pub ascendancy: Option<String>,
        /// The character level, if set.
        pub level: Option<u32>,
        /// The main skill's gem names, if the build has a main group.
        pub main_skill: Option<String>,
        /// The data version the build is pinned to.
        pub data_version: Option<DataVersionView>,
        /// `complete` or `partial`: whether anything was not read.
        pub completeness: String,
        /// `complete` or `partial` counting only what the calculation
        /// reads: lines of items in use ([`ItemSummary::in_use`]) and the
        /// custom modifier lines. A warning on an item of another item set
        /// leaves it `complete` (UX walkthrough, UX-03).
        pub in_use_completeness: String,
        /// The import's counts.
        pub lines: LineCounts,
        /// The items, in build order.
        pub items: Vec<ItemSummary>,
        /// Lines with warnings: kept, shown, not calculated (ADR-015).
        pub warnings: Vec<WarningView>,
        /// The item sets, in build order.
        pub item_sets: Vec<SetView>,
        /// Whether the active item set uses its second weapon set (the
        /// game's weapon swap, `II`).
        pub second_weapon_set: bool,
        /// The skill sets, in build order.
        pub skill_sets: Vec<SetView>,
        /// The passive tree specs, in build order.
        pub specs: Vec<SetView>,
        /// The config sets, in build order.
        pub config_sets: Vec<SetView>,
        /// The build's stages (PoB's loadouts: one per passive spec, with
        /// the item, skill, and config sets linked to it), in spec order.
        pub loadouts: Vec<LoadoutView>,
        /// The active skill set's socket groups, in order: a calc result's
        /// `gem` source names a group and a gem by their index here.
        pub skill_groups: Vec<SkillGroupView>,
        /// The names of the active passive spec's allocated nodes, when the
        /// loaded game data has them: a calc result's `passive` source
        /// names its node by number.
        pub passive_names: Vec<NodeName>,
        /// The active config set's inputs and custom modifier blocks, if
        /// the build has a config set.
        pub config: Option<ConfigView>,
        /// The league the build is planned for (`challenge` when not
        /// chosen).
        pub league: League,
        /// The active passive spec as the tree editor sees it, when the
        /// loaded data has a passive tree (passive tree plan, T5a).
        #[serde(default)]
        pub tree: Option<TreeView>,
    }

    /// The active passive spec as the tree editor sees it.
    pub struct TreeView {
        /// The spec's ID.
        pub spec: u32,
        /// The class's number (PoB's `classId`).
        pub class: u32,
        /// The ascendancy's number within the class, 0 for none.
        pub ascendancy: u32,
        /// The allocated nodes, ascending (starts included).
        pub nodes: Vec<u32>,
        /// The chosen mastery effects.
        pub masteries: Vec<MasteryChoice>,
        /// Allocated nodes the editor keeps but does not edit: ones the
        /// tree lacks or does not model (bloodline passives, cluster
        /// jewels' nodes), ascending.
        pub kept: Vec<u32>,
        /// Passive points spent (PoB's count, never enforced).
        pub points: u32,
        /// Ascendancy points spent.
        pub ascendancy_points: u32,
        /// Why the spec cannot be edited, when it cannot: its tree version
        /// differs from the loaded data's, or its class is not known. The
        /// tree is then shown, not edited.
        #[serde(default)]
        pub locked: Option<String>,
        /// The jewels socketed in the spec's sockets.
        #[serde(default)]
        pub jewels: Vec<TreeJewel>,
    }

    /// A jewel in a tree socket.
    pub struct TreeJewel {
        /// The socket node.
        pub node: u32,
        /// The item's ID.
        pub item: u32,
    }

    /// A mastery and its chosen effect.
    pub struct MasteryChoice {
        /// The mastery node.
        pub node: u32,
        /// The effect (its hash, as builds name it).
        pub effect: u32,
    }

    /// The loaded data's passive tree, for the tree screen: static while
    /// the data is loaded.
    pub struct TreeLayout {
        /// The tree version, when known.
        pub tree_version: Option<String>,
        /// Whether nodes have positions (the bundle has the layout
        /// constants); without them the tree is listed, not drawn.
        pub placed: bool,
        /// Each orbit's radius.
        pub orbit_radii: Vec<f64>,
        /// The groups' centres, by group index.
        pub groups: Vec<TreePoint>,
        /// The nodes, ascending.
        pub nodes: Vec<TreeNodeView>,
        /// The classes, by number.
        pub classes: Vec<TreeClassView>,
        /// Each mastery group's effects.
        pub masteries: Vec<MasteryGroupView>,
    }

    /// The passive tree's sprites (GGG's export sheets, ADR-005
    /// Amendment 2): static while the data is loaded. A sprite covers its
    /// pixel size divided by `zoom` in tree units.
    pub struct TreeSprites {
        /// The export's zoom level the sheets are for.
        pub zoom: f64,
        /// The sheets, each once.
        pub sheets: Vec<TreeSpriteSheet>,
        /// The export's categories (`normalActive`, `frame`, ...).
        pub categories: Vec<TreeSpriteCategory>,
        /// The nodes' art, by node.
        pub nodes: Vec<TreeNodeArt>,
    }

    /// A sheet's image.
    pub struct TreeSpriteSheet {
        /// Its file name, as the categories name it.
        pub file: String,
        /// The image as a `data:` URL (the bundle's bytes, unmodified).
        pub image: String,
    }

    /// One export category's sprites.
    pub struct TreeSpriteCategory {
        /// Its name.
        pub name: String,
        /// Its sheet's file name.
        pub file: String,
        /// Its sprites, by name.
        pub sprites: Vec<TreeSprite>,
    }

    /// One sprite on a sheet, in pixels.
    pub struct TreeSprite {
        /// Its name (an icon path, or a frame's name).
        pub name: String,
        /// Left.
        pub x: u32,
        /// Top.
        pub y: u32,
        /// Width.
        pub w: u32,
        /// Height.
        pub h: u32,
    }

    /// A node's art, as the export names it.
    pub struct TreeNodeArt {
        /// The node.
        pub node: u32,
        /// Its icon (a passive's).
        pub icon: Option<String>,
        /// A mastery's icon until it is chosen.
        pub inactive_icon: Option<String>,
        /// A mastery's icon once it is chosen.
        pub active_icon: Option<String>,
    }

    /// A point on the tree.
    pub struct TreePoint {
        /// x.
        pub x: f64,
        /// y (down is positive).
        pub y: f64,
    }

    /// What a node is.
    pub enum TreeNodeKind {
        /// A small passive.
        Normal,
        /// A notable.
        Notable,
        /// A keystone.
        Keystone,
        /// A mastery.
        Mastery,
        /// A jewel socket.
        JewelSocket,
        /// A cluster jewel socket.
        ClusterSocket,
        /// A class's start.
        ClassStart,
        /// An ascendancy's start.
        AscendancyStart,
    }

    /// One node of the tree.
    pub struct TreeNodeView {
        /// Its number.
        pub id: u32,
        /// Its name.
        pub name: String,
        /// What it is.
        pub kind: TreeNodeKind,
        /// Its group's index.
        pub group: u32,
        /// Its orbit.
        pub orbit: u32,
        /// Where it is drawn, when the tree is placed.
        pub position: Option<TreePoint>,
        /// The ascendancy it belongs to (its ID), if any.
        pub ascendancy: Option<String>,
        /// For a mastery: its mastery group's ID.
        pub mastery_group: Option<String>,
        /// Whether the editor allocates it.
        pub modelled: bool,
        /// Whether it exists only once a cluster jewel is socketed (a
        /// nested socket or position proxy): not drawn, as PoB does not.
        #[serde(default)]
        pub cluster_area: bool,
        /// The nodes it links to with a higher number (each link once).
        pub links: Vec<u32>,
        /// Its stat lines as the game prints them.
        pub lines: Vec<String>,
        /// Its flavour text, as the game shows it under a keystone's lines
        /// (the game data's; B-2 item evidence), lines split by `\n`.
        #[serde(default)]
        pub flavour: Option<String>,
    }

    /// A class on the tree.
    pub struct TreeClassView {
        /// Its number (PoB's `classId`).
        pub number: u32,
        /// Its name.
        pub name: String,
        /// Its start node.
        pub start: u32,
        /// Its ascendancies.
        pub ascendancies: Vec<TreeAscendancyView>,
    }

    /// An ascendancy on the tree.
    pub struct TreeAscendancyView {
        /// Its number within the class (PoB's `ascendClassId`).
        pub number: u32,
        /// Its ID (what nodes name).
        pub id: String,
        /// Its name.
        pub name: String,
        /// Its start node, if any.
        pub start: Option<u32>,
        /// Its quote, as the game shows it in the ascendancy's circle (the
        /// game data's; B-2 item evidence), lines split by `\n`.
        #[serde(default)]
        pub flavour: Option<String>,
        /// The quote's colour (`r,g,b`), when the data gives one.
        #[serde(default)]
        pub flavour_colour: Option<String>,
    }

    /// A mastery group's effects.
    pub struct MasteryGroupView {
        /// Its ID.
        pub id: String,
        /// Its effects.
        pub effects: Vec<MasteryEffectView>,
    }

    /// A mastery effect.
    pub struct MasteryEffectView {
        /// Its hash (what builds name).
        pub effect: u32,
        /// Its stat lines as the game prints them.
        pub lines: Vec<String>,
    }

    /// What clicking a node would do: allocate the path to it, or
    /// deallocate it and what it leaves unconnected; with each slice
    /// field before and after, when the build calculates.
    pub struct TreePreview {
        /// The node.
        pub node: u32,
        /// Nodes allocated, from the allocated ones to the node.
        pub path: Vec<u32>,
        /// Nodes deallocated, ascending.
        pub removed: Vec<u32>,
        /// Each slice field before and after (empty without a
        /// calculation).
        pub fields: Vec<FieldDelta>,
    }

    /// The active config set: its inputs and custom modifier blocks.
    pub struct ConfigView {
        /// The set's ID.
        pub set: u32,
        /// Its inputs, in build order (the name is the key).
        pub inputs: Vec<ConfigInputView>,
        /// Its custom modifier blocks, in order (the index is the key).
        pub blocks: Vec<CustomBlockView>,
    }

    /// One config input.
    pub struct ConfigInputView {
        /// The name, as the build's format writes it (PoB's option key).
        pub name: String,
        /// The value.
        pub value: ConfigValueView,
        /// Whether the value is a placeholder: the value a planner shows
        /// for an input left unset.
        pub placeholder: bool,
        /// What it does to the computed fields.
        pub effect: InputEffect,
    }

    /// One custom modifier block.
    pub struct CustomBlockView {
        /// The title, if any.
        pub title: Option<String>,
        /// Whether it applies.
        pub enabled: bool,
        /// The text as written.
        pub text: String,
        /// The modifier lines read from the text, in order.
        pub lines: Vec<CustomLineView>,
    }

    /// One line of a custom modifier block.
    pub struct CustomLineView {
        /// The text.
        pub text: String,
        /// Its status: `warning` when it did not resolve (kept, not
        /// calculated, ADR-015).
        pub status: LineStatus,
        /// The typed kind it resolved to, if any (`custom-note`: a label or
        /// comment with no effect; `custom-pattern`: read as PoB reads it).
        pub typed: Option<String>,
    }

    /// One item set, skill set, passive spec, or config set.
    pub struct SetView {
        /// The set's ID.
        pub id: u32,
        /// The title as written, if any (PoB's colour codes and `{1,2}`
        /// loadout links included).
        pub title: Option<String>,
        /// Whether the calculation uses it.
        pub active: bool,
    }

    /// A stage of the build (PoB's loadout): a passive spec and the sets
    /// linked to it, switched together.
    pub struct LoadoutView {
        /// The passive spec's ID (the stage's key).
        pub spec: u32,
        /// The title, without colour codes and links.
        pub title: String,
        /// The linked item set, if one is.
        pub item_set: Option<u32>,
        /// The linked skill set, if one is.
        pub skill_set: Option<u32>,
        /// The linked config set, if one is.
        pub config_set: Option<u32>,
        /// Whether the build uses this spec and every linked set.
        pub active: bool,
    }

    /// One socket group of the active skill set.
    pub struct SkillGroupView {
        /// The label, if any.
        pub label: Option<String>,
        /// The slot it is socketed in, if any.
        pub slot: Option<String>,
        /// Whether the group is enabled.
        pub enabled: bool,
        /// Whether it is the build's main skill group.
        pub main: bool,
        /// Its gems, in order.
        pub gems: Vec<GemView>,
    }

    /// A gem the game data offers, for a gem picker.
    pub struct GemCatalogEntry {
        /// The gem effect ID (the `gem` of [`Command::AddGem`]).
        pub id: String,
        /// The name.
        pub name: String,
        /// Whether it is a support gem.
        pub support: bool,
        /// Its colour (red, green, blue, or white), if known.
        pub colour: Option<String>,
        /// Its tag names (Spell, Area, Cold, ...).
        pub tags: Vec<String>,
        /// For a support, when a group was named: whether it would support
        /// an active gem of that group.
        pub fits: Option<bool>,
    }
    /// One gem of a socket group.
    pub struct GemView {
        /// The name as written, if any.
        pub name: Option<String>,
        /// The level.
        pub level: u32,
        /// The quality.
        pub quality: i32,
        /// Whether the gem is enabled.
        pub enabled: bool,
        /// Whether it is a support, when the game data knows the gem.
        pub support: Option<bool>,
        /// Its colour, when the game data knows it: `red`, `green`,
        /// `blue`, or `white`.
        pub colour: Option<String>,
        /// Whether it is the build's main skill (in the main group, the
        /// active skill the group's main skill names).
        pub main: bool,
        /// For a support: the gems of the group (by index) it can support,
        /// as the calculation matches supports to a skill.
        pub supports: Vec<u32>,
        /// The socket it sits in, by index in its slot's item (PoB fills a
        /// slot's sockets with its groups' gems in order); `None` when the
        /// group has no slot or the item has no socket left for it.
        pub socket: Option<u32>,
    }

    /// A passive node's name.
    pub struct NodeName {
        /// The node number.
        pub node: u32,
        /// Its name in the game data.
        pub name: String,
    }

    /// Counts of item modifier lines by status.
    pub struct LineCounts {
        /// Modifier lines.
        pub modifiers: u32,
        /// Resolved to game stats exactly.
        pub matched: u32,
        /// Resolved after an import tolerance.
        pub adapted: u32,
        /// With a warning.
        pub warnings: u32,
        /// Excluded by the item (another variant, a custom line, a
        /// section without effect).
        pub excluded: u32,
    }

    /// One item in a list.
    pub struct ItemSummary {
        /// The item's ID.
        pub id: u32,
        /// Its base's item class in the game data (`Bow`, `Quiver`, `Two
        /// Hand Sword`, `LifeFlask`, ...), when the base is known.
        pub item_class: Option<String>,
        /// The name, if any.
        pub name: Option<String>,
        /// The base, if written.
        pub base: Option<String>,
        /// `normal`, `magic`, `rare`, `unique`, `relic`, if read.
        pub rarity: Option<String>,
        /// The slot it is in, in the active item set, if any.
        pub slot: Option<String>,
        /// Its lines with warnings.
        pub warnings: u32,
        /// Whether the calculation reads it: in a slot of the active item
        /// set that is in use, or in an allocated jewel socket of the
        /// active passive spec.
        pub in_use: bool,
        /// Its sockets as linked groups, each a string of colours (`R`,
        /// `G`, `B`, `W`, `A` abyssal) in socket order: `R-R-G B` is
        /// `["RRG", "B"]`. Empty without a `Sockets` line.
        pub sockets: Vec<String>,
        /// The item sets that hold it, by ID.
        pub sets: Vec<u32>,
        /// The allocated jewel socket of the active spec it is in, by node
        /// number, if any.
        pub tree_socket: Option<u32>,
        /// Whether its selected variant is a legacy version of a unique
        /// (`Pre 3.7.0`): only Standard has such items.
        pub legacy: bool,
        /// Its requirements (level, attributes), as on its card (B-2:
        /// the equipment panel checks them against the character).
        pub requirements: Option<Requirements>,
    }

    /// One item with its lines.
    pub struct ItemView {
        /// The item's ID.
        pub id: u32,
        /// The name, if any.
        pub name: Option<String>,
        /// The base, if written.
        pub base: Option<String>,
        /// The rarity, if read.
        pub rarity: Option<String>,
        /// The slot it is in, if any.
        pub slot: Option<String>,
        /// Every source line, in order.
        pub lines: Vec<LineView>,
        /// What an item card shows (UX walkthrough, UX-08).
        pub card: ItemCard,
    }

    /// An item card's fields, from the item's header and spec lines.
    pub struct ItemCard {
        /// The item class, if written.
        pub item_class: Option<String>,
        /// The quality, as written (`20`).
        pub quality: Option<String>,
        /// The item level, as written.
        pub item_level: Option<String>,
        /// The level the item requires, as written.
        pub level_req: Option<String>,
        /// The sockets, one string of colour letters per link group.
        pub sockets: Vec<String>,
        /// The defences and other base values written on the item
        /// (`Energy Shield: 276`), in order.
        pub properties: Vec<CardProperty>,
        /// The influences, by name.
        pub influences: Vec<String>,
        /// Flag lines (`Corrupted`, `Mirrored`), as written.
        pub flags: Vec<String>,
        /// The variants' names, in order (variant 1 first).
        pub variants: Vec<String>,
        /// The selected variant (1-based), if the item names one.
        pub selected_variant: Option<u32>,
        /// What each variant is, in the order of `variants`.
        pub variant_eras: Vec<VariantEra>,
        /// When the selected variant is a legacy version: the variant the
        /// current game makes instead, if one matches.
        pub current_variant: Option<u32>,
        /// The item's mods and the lines each made, implicits first, then
        /// prefixes and suffixes, crafted last in each (the tooltip's Alt
        /// view headers): the mods PoB names, and those the lines allow
        /// one reading of; lines no mod is found for are in none.
        pub affixes: Vec<AffixView>,
        /// What the item requires, when known.
        pub requirements: Option<Requirements>,
        /// A unique's flavour text, line by line, as the game prints it
        /// under the mods (B-2): empty when the game data has none for the
        /// unique's name, or several (a unique whose variants each have
        /// their own text).
        #[serde(default)]
        pub flavour: Vec<FlavourLine>,
        /// The item's own values as the calculation computes them (B-2
        /// item evidence): an armour piece's defences, a shield's block, a
        /// weapon's damage, rate, and critical strike chance. None for an
        /// item without such values, or when the data or the base is
        /// missing.
        #[serde(default)]
        pub values: Option<CardValues>,
        /// The most sockets the item's base rolls at its item level (the
        /// game data's item types; B-2 item evidence). None when the data
        /// has no caps for the base. More sockets than this is shown on the
        /// card, never a calculation state.
        #[serde(default)]
        pub max_sockets: Option<u32>,
    }

    /// An item's own values ([`ItemCard::values`]), each as the game's
    /// tooltip shows it before rounding for display.
    pub struct CardValues {
        /// The defences above 0, in the game's order (armour, evasion,
        /// energy shield).
        pub defences: Vec<CardDefence>,
        /// A shield's chance to block, percent.
        pub block: Option<f64>,
        /// A weapon's damage per type, physical first.
        pub damage: Vec<CardDamage>,
        /// A weapon's attacks per second.
        pub attack_rate: Option<f64>,
        /// A weapon's critical strike chance, percent.
        pub crit_chance: Option<f64>,
        /// The values leave out something the engine does not model on
        /// this item (an unmodelled line or base, missing data).
        pub partial: bool,
    }

    /// One defence on an item card.
    pub struct CardDefence {
        /// `armour`, `evasion`, or `energy-shield`.
        pub defence: String,
        /// The value.
        pub value: f64,
    }

    /// One damage type's range on an item card.
    pub struct CardDamage {
        /// `physical`, `fire`, `cold`, `lightning`, or `chaos`.
        pub damage_type: String,
        /// The minimum.
        pub min: f64,
        /// The maximum.
        pub max: f64,
    }

    /// One line of a unique's flavour text. An empty line is the gap the
    /// game leaves between parts.
    pub struct FlavourLine {
        /// The text, the game's markup removed.
        pub text: String,
        /// The game prints it in the default text colour (`<default>{...}`:
        /// a note such as "This item can be transformed on the Altar of
        /// Sacrifice"), not the flavour colour.
        pub default_colour: bool,
    }

    /// One mod of an item, as the tooltip's Alt view heads its lines
    /// (`Prefix Modifier "Urchin's" (Tier: 2) — Life, Defences`).
    pub struct AffixView {
        /// Prefix or suffix.
        pub kind: AffixKind,
        /// The affix name, when the game data has one.
        pub name: Option<String>,
        /// Its tier on the item's base (1 is the best), when known.
        pub tier: Option<u32>,
        /// Whether it is crafted (`Master Crafted Prefix Modifier`).
        pub crafted: bool,
        /// Whether it was inferred from the lines (the only reading the
        /// game data allows) rather than named by the item.
        pub inferred: bool,
        /// Its tags as the game prints them (`Life`, `Defences`).
        pub tags: Vec<String>,
        /// The line indices it made, in item order.
        pub lines: Vec<u32>,
    }

    /// An item's requirements: the level, and attributes above 0.
    pub struct Requirements {
        /// The level: as written on the item, else the base's (its drop
        /// level for a weapon, armour, or flask, its implicits' levels; PoB's
        /// rule, `loom_data::bundle::required_level`).
        pub level: Option<u32>,
        /// Strength (0: none).
        pub strength: u32,
        /// Dexterity (0: none).
        pub dexterity: u32,
        /// Intelligence (0: none).
        pub intelligence: u32,
    }

    /// One base value on an item card.
    pub struct CardProperty {
        /// The name (`Energy Shield`).
        pub name: String,
        /// The value, as written.
        pub value: String,
    }

    /// One source line of an item.
    pub struct LineView {
        /// The line's index among the item's lines.
        pub index: u32,
        /// The line as written.
        pub text: String,
        /// What it is: the domain's line kind name (`modifier`, `spec`,
        /// `name`, ...).
        pub kind: String,
        /// The modifier list it belongs to (`implicit`, `explicit`, ...);
        /// modifier lines only.
        pub section: Option<String>,
        /// Its status.
        pub status: LineStatus,
        /// The warning's code (`LINE_NO_TEMPLATE`, ...), when the status is
        /// `warning`.
        pub code: Option<String>,
        /// What the line resolved to: game stats, or a typed kind.
        pub stats: Vec<StatView>,
        /// The typed kind (`granted-skill`, ...), if it resolved to one.
        pub typed: Option<String>,
        /// The tolerance it needed (`case-folded`, `split-section`).
        pub adapted: Option<String>,
        /// The numbers in the text, in order, for roll edits.
        pub numbers: Vec<NumberSpan>,
        /// A modifier line as the game prints it, its roll applied (no
        /// markup); `None` when it is shown as written.
        pub shown: Option<String>,
        /// The line as the game prints it with Alt held: each roll followed
        /// by its range (`+28(24-28) to maximum Life`); `None` when the
        /// line has no range.
        pub ranged: Option<String>,
        /// The variants (1-based) a modifier line belongs to; empty for
        /// every variant.
        pub variants: Vec<u32>,
        /// The modifier's marks from its prefixes: `crafted`,
        /// `fractured`, `crucible`, `scourge`, `synthesis`, `mutated`.
        pub marks: Vec<String>,
    }

    /// A number in a line's text: where it is, as character offsets.
    pub struct NumberSpan {
        /// The first character's offset.
        pub start: u32,
        /// The offset after the last character.
        pub end: u32,
        /// The number as written.
        pub text: String,
    }

    /// One stat a line resolved to.
    pub struct StatView {
        /// The game's stat ID.
        pub id: String,
        /// The value as text (`15`, `10 to 20`, `hidden`).
        pub value: String,
    }

    /// A line with a warning.
    pub struct WarningView {
        /// The item's ID.
        pub item: u32,
        /// The item's name, if any.
        pub item_name: Option<String>,
        /// The line's index.
        pub line: u32,
        /// The line as written.
        pub text: String,
        /// The stable code.
        pub code: String,
        /// Whether the line's item is in use ([`ItemSummary::in_use`]): a
        /// warning on an item not in use does not touch the calculation.
        pub in_use: bool,
    }

    /// Undo and redo.
    pub struct HistoryView {
        /// Whether there is a change to undo.
        pub can_undo: bool,
        /// Whether there is a change to redo.
        pub can_redo: bool,
        /// What undo would revert.
        pub undo: Option<HistoryEntry>,
        /// What redo would apply again.
        pub redo: Option<HistoryEntry>,
    }

    /// One change in the history (ADR-017: an addon's change is recorded
    /// like the user's).
    pub struct HistoryEntry {
        /// What it was.
        pub kind: ChangeKind,
        /// The item it changed (for `item-set`: the item set it made
        /// active).
        pub item: u32,
        /// The change request it applied, when an MCP client asked for it.
        pub request: Option<u64>,
    }

    /// The calculation of the latest version.
    pub struct CalcStatus {
        /// Where it is.
        pub state: CalcState,
        /// The version of the result held (`result` is of it); may be
        /// older than the snapshot's while a calculation runs.
        pub version: u64,
        /// The result's overall state (`complete`, `partial`,
        /// `unsupported`, `indeterminate`), when there is one.
        pub result_state: Option<String>,
        /// Why the latest calculation failed, if it did.
        pub error: Option<ApiError>,
    }

    /// An item edit: line operations on one item. It is refused as stale
    /// (`APP_STALE_EDIT`) when the item's lines it names are not the ones
    /// it expects.
    pub struct ItemEdit {
        /// The item's ID.
        pub item: u32,
        /// The operations, in order; line indices are the item's before
        /// the edit.
        pub ops: Vec<LineOp>,
        /// The text each line the operations name had when the edit was
        /// made, by line index.
        pub expect: Vec<ExpectedLine>,
    }

    /// A line's text an edit expects.
    pub struct ExpectedLine {
        /// The line's index.
        pub line: u32,
        /// Its text.
        pub text: String,
    }

    /// An item edit's effect, calculated by the application (never by the
    /// requester).
    pub struct ItemEditPreview {
        /// The item as edited, with its lines' new statuses.
        pub item: ItemView,
        /// Each slice field before and after.
        pub fields: Vec<FieldDelta>,
    }

    /// One field before and after an edit.
    pub struct FieldDelta {
        /// The field's name.
        pub field: String,
        /// Its value before, when computed and finite.
        pub before: Option<f64>,
        /// Its value after, when computed and finite.
        pub after: Option<f64>,
        /// Its state before.
        pub state_before: String,
        /// Its state after.
        pub state_after: String,
    }

    /// The MCP server (ADR-017).
    pub struct McpStatus {
        /// Whether it runs.
        pub running: bool,
        /// Its address, while it runs (`http://127.0.0.1:<port>/mcp`).
        pub url: Option<String>,
        /// The bearer token, while it runs.
        pub token: Option<String>,
        /// Whether the token is kept between starts in the OS's protected
        /// store (ADR-017 Amendment 1); `false` when it is made at each
        /// start.
        #[serde(default)]
        pub token_kept: bool,
        /// What clients may do.
        pub permissions: McpPermissions,
        /// Change requests, newest last.
        pub requests: Vec<ChangeRequest>,
    }

    /// What MCP clients may do. Reading is allowed while the server runs.
    pub struct McpPermissions {
        /// Clients may request changes (off by default: read-only).
        pub requests: bool,
        /// Requests apply without asking (a standing grant the user can
        /// revoke).
        pub standing_grant: bool,
    }

    /// A change an MCP client asked for.
    pub struct ChangeRequest {
        /// Its ID, unique in the session.
        pub id: u64,
        /// The client's name, as it gave it.
        pub client: String,
        /// The item edit, for an item request; `null` for a tree request.
        pub edit: Option<ItemEdit>,
        /// The client's note, if any.
        pub note: Option<String>,
        /// The item edit's effect, calculated when it arrived.
        pub preview: Option<ItemEditPreview>,
        /// Where it is.
        pub status: RequestStatus,
        /// Why it failed, if it did.
        pub error: Option<ApiError>,
        /// The passive tree change, for a tree request.
        #[serde(default)]
        pub tree: Option<TreeChange>,
        /// The tree change's effect, calculated when it arrived.
        #[serde(default)]
        pub tree_preview: Option<TreePreview>,
    }

    /// A passive tree change an MCP client asks for: the tree commands
    /// of the same names.
    pub enum TreeChange {
        /// Allocates a node and the shortest path to it.
        AllocateNode {
            /// The spec's ID.
            spec: u32,
            /// The node.
            node: u32,
        },
        /// Deallocates a node and the nodes it leaves unconnected.
        DeallocateNode {
            /// The spec's ID.
            spec: u32,
            /// The node.
            node: u32,
        },
        /// Chooses a mastery's effect, allocating the mastery.
        SetMasteryEffect {
            /// The spec's ID.
            spec: u32,
            /// The mastery node.
            node: u32,
            /// The effect.
            effect: u32,
        },
    }

    /// A recovery snapshot (autosave) found at start.
    pub struct RecoveryEntry {
        /// Its ID (the file's stem).
        pub id: String,
        /// When it was written, in seconds since the Unix epoch.
        pub saved_at: u64,
        /// The build's display name.
        pub name: String,
        /// Where the build was saved, if it was.
        pub path: Option<String>,
        /// The file it was imported from, if any.
        pub origin: Option<String>,
    }

    /// A recent build.
    pub struct RecentEntry {
        /// The ID the application issued for it (the UI opens a recent
        /// build by it and never sends a path).
        pub id: String,
        /// The build document's path.
        pub path: String,
        /// The display name.
        pub name: String,
        /// When it was last opened or saved, in seconds since the Unix
        /// epoch.
        pub at: u64,
        /// The class, if known (UX-20; lists written before it have none).
        pub class: Option<String>,
        /// The ascendancy, if known.
        pub ascendancy: Option<String>,
        /// The character level, if known.
        pub level: Option<u32>,
        /// The main skill, if known.
        pub main_skill: Option<String>,
    }

    /// An error: a stable code (ADR-014) and a message for logs. The UI
    /// shows text for the code, not the message.
    pub struct ApiError {
        /// The code of the layer that failed (`IMPORT_*`, `PERSIST_*`,
        /// `APP_*`, ...).
        pub code: String,
        /// Detail in English, for diagnostics.
        pub message: String,
    }

    /// An event (ADR-004): something committed, a job's progress, or an
    /// error.
    pub struct Event {
        /// The snapshot version after the event.
        pub version: u64,
        /// The command that caused it, if any.
        pub correlation: Option<u64>,
        /// What happened.
        pub kind: EventKind,
    }
}

/// Declares DTO enums: externally tagged, kebab-case.
macro_rules! dto_enum {
    ($($item:item)*) => {
        $(
            #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
            #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
            #[serde(deny_unknown_fields, rename_all = "kebab-case")]
            $item
        )*
    };
}

dto_enum! {
    /// A line's status, as the UI names it (`docs/ui/UI-DESIGN-BRIEF.md`,
    /// terminology).
    pub enum LineStatus {
        /// Resolved to game stats or a typed kind.
        Matched,
        /// Resolved after an import tolerance.
        Adapted,
        /// Kept and shown, not calculated (ADR-015).
        Warning,
        /// Excluded by the item.
        Excluded,
        /// Continues the line before it (one description over several
        /// lines).
        Continues,
        /// Not a modifier line.
        Other,
    }

    /// A calculation's progress.
    pub enum CalcState {
        /// No build, or no data.
        Idle,
        /// A calculation of the latest version runs.
        Running,
        /// The result is of the latest version.
        Done,
        /// The latest version's calculation failed.
        Failed,
    }

    /// What a change was, for undo and redo labels.
    pub enum ChangeKind {
        /// An item edit.
        ItemEdit,
        /// An item edit an MCP client requested.
        RequestApplied,
        /// Another item set made active.
        ItemSet,
        /// Another stage (loadout) made active.
        Loadout,
        /// Another skill set made active.
        SkillSet,
        /// A gem or socket group changed, or the main skill.
        SkillEdit,
        /// Another config set made active, or a config input or custom
        /// modifier block changed.
        Config,
        /// The league the build is planned for changed.
        League,
        /// The active item set's weapon set switched (weapon swap).
        WeaponSet,
        /// A passive tree edit: a node, a mastery effect, the class, the
        /// ascendancy, or a reset.
        Tree,
        /// A passive tree added, copied, renamed, or deleted.
        TreeSpecs,
        /// A passive tree change an MCP client requested.
        TreeRequestApplied,
        /// An item added (and perhaps put in a slot).
        ItemAdded,
    }

    /// The league a build is planned for.
    pub enum League {
        /// A challenge league: legacy versions of unique items cannot be
        /// had.
        Challenge,
        /// Standard: items keep their legacy versions.
        Standard,
    }

    /// Where an affix is on the item.
    pub enum AffixKind {
        /// An implicit modifier.
        Implicit,
        /// A prefix.
        Prefix,
        /// A suffix.
        Suffix,
    }

    /// What a variant of an item is.
    pub enum VariantEra {
        /// A legacy version (`Pre 3.7.0`): only items from earlier leagues.
        Legacy,
        /// The version the current game makes (`Current`).
        Current,
        /// A choice the player makes (`Fire`, `Life`), not a version.
        Choice,
    }

    /// A config input's value.
    pub enum ConfigValueView {
        /// A flag.
        Boolean(bool),
        /// A number.
        Number(f64),
        /// Text.
        Text(String),
    }

    /// What a config input does to the computed fields.
    pub enum InputEffect {
        /// The calculation applies it.
        Applied,
        /// It changes no computed field.
        NoEffect,
        /// The calculation does not model it: computed fields are partial.
        Unmodelled,
    }

    /// One line operation of an item edit.
    pub enum LineOp {
        /// Replaces the line's text.
        Replace {
            /// The line's index.
            line: u32,
            /// The new text.
            text: String,
        },
        /// Removes the line.
        Remove {
            /// The line's index.
            line: u32,
        },
        /// Sets a ranged line's roll, as PoB's `{range:r}` prefix (0 the
        /// low end, 1 the high end), keeping the range.
        SetRange {
            /// The line's index.
            line: u32,
            /// The roll, from 0 to 1.
            value: f64,
        },
        /// Selects the item's variant (1-based): its `Selected Variant`
        /// line is written (added after the variants when missing), and
        /// the item is read again.
        SelectVariant {
            /// The variant.
            variant: u32,
        },
        /// Adds a modifier line after the item's last line.
        Add {
            /// The text.
            text: String,
        },
    }

    /// A change request's status.
    pub enum RequestStatus {
        /// Waiting for the user.
        Pending,
        /// Applied (approved, or under the standing grant).
        Applied,
        /// Declined by the user.
        Declined,
        /// It could not apply (the item changed, a line is gone, ...).
        Failed,
    }

    /// A command (ADR-004): an intent the application runs in order.
    pub enum Command {
        /// Starts a new PoE1 build: Scion, level 1, empty sets (UX brief
        /// note 3).
        NewBuild,
        /// Loads an installed bundle by its ID (B-1) and remembers the
        /// choice.
        UseBundle {
            /// The bundle's ID ([`InstalledBundle`]).
            id: String,
        },
        /// Accepts a quarantined bundle (Phase 5, P-e): the user saw its
        /// manifest's SHA-256 and approved it. Refused when the bundle
        /// changed since (`APP_QUARANTINED`).
        AcceptBundle {
            /// The bundle's ID ([`QuarantinedBundle`]).
            id: String,
            /// The manifest SHA-256 the user was shown.
            #[serde(rename = "manifest-sha256")]
            manifest_sha256: String,
        },
        /// Checks every file of an installed bundle against its manifest's
        /// hashes (the user's "verify data"; the start checks sizes only).
        /// `APP_BUNDLE_DAMAGED` when one differs.
        VerifyBundle {
            /// The bundle's ID.
            id: String,
        },
        /// Imports a PoB share code or XML text.
        ImportText {
            /// The text.
            text: String,
        },
        /// Applies an item edit.
        EditItem(ItemEdit),
        /// Adds an item to the build from the game's item copy (Ctrl+C or
        /// Ctrl+Alt+C) or PoB's item text, as one undoable change; with
        /// `slot`, also puts it in that slot of the active item set (B-2
        /// item evidence, E3).
        AddItem {
            /// The copied text.
            text: String,
            /// A slot of the active item set (`Helmet`, `Weapon 1`,
            /// `Ring 2`, `Flask 3`, ...), if the item goes in one.
            slot: Option<String>,
        },
        /// Makes an item set the active one, as one undoable change.
        UseItemSet {
            /// The set's ID.
            id: u32,
        },
        /// Makes a stage (loadout) the active one: its passive spec and the
        /// sets linked to it, as one undoable change.
        UseLoadout {
            /// The passive spec's ID.
            spec: u32,
        },
        /// Makes a skill set the active one, as one undoable change.
        UseSkillSet {
            /// The set's ID.
            id: u32,
        },
        /// Changes a gem of the active skill set, as one undoable change.
        EditGem {
            /// The socket group's index in the active skill set.
            group: u32,
            /// The gem's index in the group.
            gem: u32,
            /// A new level.
            level: Option<u32>,
            /// A new quality.
            quality: Option<i32>,
            /// Enables or disables the gem.
            enabled: Option<bool>,
        },
        /// Changes a socket group of the active skill set, as one undoable
        /// change.
        EditSocketGroup {
            /// The group's index in the active skill set.
            group: u32,
            /// Enables or disables the group.
            enabled: Option<bool>,
            /// Makes it the main skill group (`true` only).
            main: Option<bool>,
            /// Makes this gem (by index in the group) the group's main active
            /// skill; with `main`, the build's main skill.
            #[serde(rename = "main-gem")]
            main_gem: Option<u32>,
            /// Moves the group to this equipment slot (empty: no slot).
            slot: Option<String>,
            /// Renames the group; empty for no label.
            label: Option<String>,
        },
        /// Adds a gem to a socket group of the active skill set.
        AddGem {
            /// The group's index in the active skill set.
            group: u32,
            /// The gem, by its gem effect ID (a [`GemCatalogEntry`]'s `id`).
            gem: String,
            /// Its place in the group (by index); the end when absent.
            at: Option<u32>,
        },
        /// Removes a gem from a socket group of the active skill set.
        RemoveGem {
            /// The group's index in the active skill set.
            group: u32,
            /// The gem's index in the group.
            gem: u32,
        },
        /// Adds an empty socket group at the end of the active skill set.
        AddSocketGroup {
            /// The equipment slot it is socketed in, if any.
            slot: Option<String>,
            /// Its label, if any.
            label: Option<String>,
        },
        /// Removes a socket group of the active skill set.
        RemoveSocketGroup {
            /// The group's index in the active skill set.
            group: u32,
        },
        /// Makes a config set the active one, as one undoable change.
        UseConfigSet {
            /// The set's ID.
            id: u32,
        },
        /// Sets a config input of the active config set (added when the
        /// set has none of that name; a placeholder becomes a value), as
        /// one undoable change.
        SetConfigInput {
            /// The name (PoB's option key).
            name: String,
            /// The value.
            value: ConfigValueView,
        },
        /// Removes a config input of the active config set.
        RemoveConfigInput {
            /// The name.
            name: String,
        },
        /// Changes a custom modifier block of the active config set; a new
        /// text's lines are read and resolved again.
        EditCustomBlock {
            /// The block's index.
            block: u32,
            /// Whether it applies, if changed.
            enabled: Option<bool>,
            /// The new text, if changed.
            text: Option<String>,
        },
        /// Adds a custom modifier block, with `text`, after the others.
        AddCustomBlock {
            /// The text.
            text: String,
        },
        /// Removes a custom modifier block.
        RemoveCustomBlock {
            /// The block's index.
            block: u32,
        },
        /// Switches the active item set to its first or second weapon set
        /// (the game's weapon swap), as one undoable change.
        UseWeaponSet {
            /// The second weapon set (`II`) when set.
            second: bool,
        },
        /// Sets the league the build is planned for, as one undoable
        /// change.
        SetLeague {
            /// The league.
            league: League,
        },
        /// Allocates a node of a passive spec and the path to it (the
        /// path is the application's, never sent), as one undoable
        /// change.
        AllocateNode {
            /// The spec's ID.
            spec: u32,
            /// The node.
            node: u32,
        },
        /// Deallocates a node and every node it leaves unconnected.
        DeallocateNode {
            /// The spec's ID.
            spec: u32,
            /// The node.
            node: u32,
        },
        /// Sets a mastery's effect, allocating the mastery.
        SetMasteryEffect {
            /// The spec's ID.
            spec: u32,
            /// The mastery node.
            node: u32,
            /// The effect's hash.
            effect: u32,
        },
        /// Changes a spec's class: no ascendancy, nodes no longer
        /// connected deallocated (the UI confirms first).
        SelectClass {
            /// The spec's ID.
            spec: u32,
            /// The class's number.
            class: u32,
        },
        /// Changes a spec's ascendancy (0 for none): the old one's nodes
        /// deallocated.
        SelectAscendancy {
            /// The spec's ID.
            spec: u32,
            /// The ascendancy's number within the class.
            ascendancy: u32,
        },
        /// Deallocates every node of a spec but the starts, kept ones
        /// too.
        ResetTree {
            /// The spec's ID.
            spec: u32,
        },
        /// Sockets a jewel of the build in an allocated jewel socket of a
        /// spec, or empties the socket (`item` absent). Cluster jewels
        /// are refused for now.
        SocketJewel {
            /// The spec's ID.
            spec: u32,
            /// The socket node.
            node: u32,
            /// The item's ID; absent to empty the socket.
            #[serde(default)]
            item: Option<u32>,
        },
        /// Adds a passive tree at the end of the list (a stage of its
        /// own): a copy of spec `copy`, or a new one with the active
        /// spec's class and ascendancy and only the starts. It does not
        /// become active.
        NewSpec {
            /// Its name.
            title: String,
            /// The spec it copies, if any.
            #[serde(default)]
            copy: Option<u32>,
        },
        /// Renames a passive tree.
        RenameSpec {
            /// The spec's ID.
            spec: u32,
            /// Its new name.
            title: String,
        },
        /// Deletes a passive tree; never the last one. The specs after it
        /// are renumbered; when it was active, the one before it becomes
        /// active.
        DeleteSpec {
            /// The spec's ID.
            spec: u32,
        },
        /// Reverts the latest change.
        Undo,
        /// Applies the latest reverted change again.
        Redo,
        /// Saves the build where it is saved.
        Save,
        /// Opens a recent build by its ID.
        OpenRecent {
            /// The ID.
            id: String,
        },
        /// Removes a build from the recent list (the file stays).
        ForgetRecent {
            /// The ID.
            id: String,
        },
        /// Restores a recovery snapshot.
        Restore {
            /// The snapshot's ID.
            id: String,
        },
        /// Discards a recovery snapshot.
        Discard {
            /// The snapshot's ID.
            id: String,
        },
        /// Approves a change request.
        Approve {
            /// The request's ID.
            id: u64,
        },
        /// Declines a change request.
        Decline {
            /// The request's ID.
            id: u64,
        },
        /// Writes the build's PoB share code.
        ExportShareCode,
        /// Closes the build.
        Close,
    }

    /// A command's answer.
    pub enum Outcome {
        /// Done; the snapshot version after it.
        Done {
            /// The version.
            version: u64,
        },
        /// Done, with text to give the user (a share code).
        Text {
            /// The version.
            version: u64,
            /// The text.
            text: String,
        },
        /// Nothing to do (undo with an empty history, a cancelled dialog).
        Nothing,
    }

    /// What an event reports.
    pub enum EventKind {
        /// The build changed (or was opened, imported, closed).
        BuildChanged,
        /// The build was saved.
        Saved {
            /// Where.
            path: String,
        },
        /// A calculation started.
        CalcStarted,
        /// A calculation finished; its result is the snapshot's.
        CalcFinished {
            /// The result's overall state.
            state: String,
        },
        /// A calculation failed.
        CalcFailed {
            /// Why.
            error: ApiError,
        },
        /// The MCP server or its requests changed.
        McpChanged,
        /// A command failed.
        Failed {
            /// Why.
            error: ApiError,
        },
    }
}

impl ApiError {
    /// An error with this code and message.
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_should_read_its_kebab_case_wire_form() {
        let json = r#"{"edit-item":{"item":1,"ops":[{"replace":{"line":3,"text":"+1 to x"}},"undo-is-not-an-op"]}}"#;
        assert!(serde_json::from_str::<Command>(json).is_err());
        let json = r#"{"edit-item":{"item":1,"ops":[{"replace":{"line":3,"text":"+1 to x"}},{"remove":{"line":4}},{"add":{"text":"y"}}],"expect":[]}}"#;
        let command: Command = serde_json::from_str(json).unwrap();
        assert_eq!(
            command,
            Command::EditItem(ItemEdit {
                item: 1,
                ops: vec![
                    LineOp::Replace {
                        line: 3,
                        text: "+1 to x".to_owned()
                    },
                    LineOp::Remove { line: 4 },
                    LineOp::Add {
                        text: "y".to_owned()
                    },
                ],
                expect: Vec::new(),
            })
        );
        assert_eq!(serde_json::to_string(&Command::Undo).unwrap(), r#""undo""#);
    }

    #[test]
    fn event_should_write_kebab_case_fields() {
        let event = Event {
            version: 3,
            correlation: Some(7),
            kind: EventKind::CalcFinished {
                state: "partial".to_owned(),
            },
        };
        assert_eq!(
            serde_json::to_string(&event).unwrap(),
            r#"{"version":3,"correlation":7,"kind":{"calc-finished":{"state":"partial"}}}"#
        );
    }
}
