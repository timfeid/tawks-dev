/** Rust for the current definition: the template's own builder code while it is
 * unchanged, otherwise code that loads the exact JSON the playground shows. */
export function rustFor(definition: unknown, templateSource: string | null): string {
  if (templateSource) return templateSource;
  const json = JSON.stringify(definition, null, 2);
  let hashes = '#';
  while (json.includes(`"${hashes}`)) hashes += '#';
  return `use tawks_bot::workflow::CreateWorkflowDefinition;

/// Built in the tawks playground. Register it with \`bot.register_workflow(definition())\`.
pub fn definition() -> CreateWorkflowDefinition {
    serde_json::from_str(DEFINITION).expect("a valid workflow definition")
}

const DEFINITION: &str = r${hashes}"${json}"${hashes};
`;
}
