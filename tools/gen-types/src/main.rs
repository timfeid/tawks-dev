//! Regenerates web/workflow-types/index.d.ts from the Rust workflow model, so
//! the renderer and the playground read the same shapes bots write, and the
//! playground's palette and templates.
//!
//!   cargo run -p tawks-gen-types

mod playground;
mod templates;

use specta::TypeCollection;
use specta_typescript::Typescript;
use tawks_bot::types::UserResource;
use workflow_model::service::{WorkflowResource, WorkflowResourceWithContext};
use workflow_model::{
    ActionType, CreateWorkflowDefinition, DisplayType, InputType, SelectOption, WorkflowAction,
    WorkflowDefinition, WorkflowDisplay, WorkflowInput, WorkflowNode,
};

fn main() {
    let mut types = TypeCollection::default();
    types
        .register::<CreateWorkflowDefinition>()
        .register::<WorkflowDefinition>()
        .register::<WorkflowNode>()
        .register::<WorkflowDisplay>()
        .register::<WorkflowInput>()
        .register::<WorkflowAction>()
        .register::<ActionType>()
        .register::<DisplayType>()
        .register::<InputType>()
        .register::<SelectOption>()
        .register::<WorkflowResource>()
        .register::<WorkflowResourceWithContext>()
        .register::<UserResource>();

    let out = concat!(env!("CARGO_MANIFEST_DIR"), "/../../web/workflow-types/index.d.ts");
    Typescript::default().export_to(out, &types).expect("failed to write the workflow types");
    println!("wrote {out}");

    let out = concat!(env!("CARGO_MANIFEST_DIR"), "/../../site/src/lib/playground/generated.json");
    let json = serde_json::to_string_pretty(&playground::build()).expect("playground data serializes");
    std::fs::write(out, json + "\n").expect("failed to write the playground data");
    println!("wrote {out}");
}
