//! Remix icons, generated from `packages/ui/src/components/icon/sprite.ts`; do not edit.
//!
//! Do not edit by hand: `node tools/gen-remix-icons.mjs` rewrites this file and the SVG
//! files it points at. Names are React's sprite symbols, one per icon file. The sprite order
//! lives in `remix_all.rs` and the name match in `remix_names.rs`.

use gpui_kit::SharedString;
use gpui_kit::component::{Icon, IconNamed};
use gpui_kit::{App, IntoElement, RenderOnce, Window};

#[path = "remix_all.rs"]
mod remix_all;
#[path = "remix_names.rs"]
mod remix_names;

/// One symbol of React's Remix sprite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, IntoElement)]
#[rustfmt::skip]
pub(crate) enum RemixIcon {
    Add, AddCircle, AiAgent, AiAgentFill, AiGenerate2, AlarmWarning, Alert, Android,
    AppAndroidStudio, AppAntigravity, AppCursor, AppEclipse, AppGhostty, AppIntellij, AppIterm2, AppKiro,
    AppPhpstorm, AppPycharm, AppRider, AppRustrover, AppSublimeText, AppTrae, AppVisualStudio, AppVscode,
    AppVscodium, AppWebstorm, AppWindsurf, AppXcode, AppZed, Apple, Apps, Archive,
    ArchiveStack, ArrowDown, ArrowDownS, ArrowGoBack, ArrowLeft, ArrowLeftRight, ArrowLeftS, ArrowRight,
    ArrowRightS, ArrowUp, ArrowUpDouble, ArrowUpDown, ArrowUpS, Attachment2, Bank, BankCard,
    BarChart, BarChart2, BarChartBox, Book, BookMarked, BookOpen, Booklet, Box3,
    Braces, Brain, Brain4, BrainAi3, Briefcase, Brush, Bug, Calendar,
    CalendarEvent, CalendarSchedule, Camera, Chat1, Chat3, Chat4, ChatAi3, ChatCheck,
    ChatForward, ChatHistory, ChatNew, ChatOff, ChatThread, Check, CheckDouble, Checkbox,
    CheckboxBlank, CheckboxBlankCircle, CheckboxBlankCircleFill, CheckboxCircle, CheckboxMultiple, Circle, Clipboard, Close,
    CloseCircle, Cloud, CloudOff, Cloudflare, Code, CodeAi, CodeBox, CodeSslash,
    CollapseVertical, Command, Compass3, Computer, ContactsBook, ContractUpDown, CornerDownLeft, CornerDownRight,
    Cpu, Cup, CustomerService2, Database2, DeleteBin, DeleteBin2, DiscordFill, Discuss,
    DonutChart, DonutChartFill, DoubleQuotesL, Download, DownloadCloud, DownloadCloud2, DragMove2, Draggable,
    Edit, Edit2, EmotionHappy, Equalizer2, ErrorWarning, ExpandHorizontal, ExpandLeft, ExpandRight,
    ExpandUpDown, ExternalLink, Eye, EyeOff, Feedback, File, FileAdd, FileCode,
    FileCopy, FileCopy2, FileEdit, FileForbid, FileImage, FileList2, FileMusic, FilePaper2,
    FilePdf, FileSearch, FileText, FileUpload, FileVideo, Film, Filter3, Finder,
    Fingerprint, Fire, FireFill, Flag, Flashlight, Flask, FlowChart, Focus3,
    Folder, Folder3, Folder3Fill, Folder6, FolderAdd, FolderOpen, FolderOpenFill, FolderReceived,
    FolderShared, Folders, Fullscreen, FullscreenExit, Gamepad, Gift, GitBranch, GitClosePullRequest,
    GitCommit, GitMerge, GitPrDraft, GitPullRequest, GitRepository, Github, GithubFill, Gitlab,
    Global, GraduationCap, Hammer, HardDrive2, Hashtag, Heart, History, Home,
    HomeOffice, Image, Image2, ImageDownload, InboxArchive, InboxUnarchive, InboxUnarchiveFill, Information,
    Instance, Key, Keyboard, Layout, LayoutBottom, LayoutColumn, LayoutLeft, LayoutRight,
    Leaf, Lightbulb, LineChart, Linear, Link, ListCheck2, ListIndefinite, ListUnordered,
    Loader, Loader4, Lock, LockUnlock, LoopLeft, LoopRight, LoopRightAi, Macbook,
    Mail, MapPin2, Markup, Menu2, MenuSearch, Mic, MoneyDollarCircle, Moon,
    More, More2, More2Fill, MoreFill, Music, NodeTree, Notification3, Npmjs,
    Openchamber, Opencode, Palette, Pause, PenNib, Pencil, PencilAi, PencilAi2,
    PencilRuler2, PictureInPicture2, PieChart, Plane, Planet, Play, Plug, Plug2,
    Pulse, Pushpin, Pushpin2, Pushpin2Fill, Puzzle, Puzzle2, Question, QuillPen,
    RecordCircle, Refresh, Restart, Robot, Robot2, Rocket, Route, Save3,
    Scales3, Scan2, Scissors, Search, SearchEye, SendPlane, SendPlane2, Server,
    Settings3, Shield, ShieldCheck, ShieldKeyhole, ShieldStar, ShieldUser, Shuffle, Signpost,
    SlashCommands2, Smartphone, Space, Sparkling, Sparkling2, SpeedUp, SplitCellsHorizontal, Stack,
    Star, StarFill, StickyNote, Stop, StopLarge, Store2, Subtract, Sun,
    Survey, Table2, Target, TargetFill, Task, Team, Terminal, TerminalBox,
    TerminalWindow, TestTube, Text, TextWrap, ThumbUp, Ticket2, Time, Timer,
    Tools, Trophy, TwitterXfill, Ubuntu, Unpin, Upload2, UploadCloud2, User,
    User3, UserSearch, VerifiedBadge, VolumeUp, Window, Windows, Worktree, ZoomIn,
    ZoomOut,
}

impl IconNamed for RemixIcon {
    #[rustfmt::skip]
    fn path(self) -> SharedString {
        format!("icons/remix/{}.svg", self.name()).into()
    }
}

impl RenderOnce for RemixIcon {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        Icon::new(self)
    }
}
