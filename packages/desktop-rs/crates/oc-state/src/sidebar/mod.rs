//! The sidebar as a pure derived view of settings, home info and the global session list.
//!
//! React builds the same view in `SessionProjectCollection.tsx` from
//! `sessionCollection.ts`, `sectionMembership.ts`, `sidebarWorks.ts`, `projectSort.ts` and
//! `sessionSidebarRowModel.ts`. Here the reducer only stores the inputs; [`SidebarModel`] is
//! computed on demand, so a page, a settings toggle and an event cannot disagree about order.

mod build;
mod membership;
mod model;
mod order;
mod section;
mod split;

pub(crate) use build::sections;
pub use model::{
    MoreKey, ProjectGroup, SidebarModel, SidebarProject, SidebarRow, SidebarSectionModel,
};
pub(crate) use split::{ChatRoots, project_index};
