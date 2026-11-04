import { test } from "node:test";
import assert from "node:assert/strict";
import {
  blockKeys,
  duplicateAt,
  editableFields,
  getAt,
  isBound,
  moveBy,
  moveSummary,
  moveTo,
  pageTree,
  removeAt,
  setField,
  type Json,
  type Path,
} from "./tree.ts";

const text = (words: string, role = "Body") => ({
  type: { Typography: { text: words, role, tone: "" } },
});
const section = (
  title: string,
  content: Json[],
  eyebrow: string | null = null,
) => ({
  type: { Section: { eyebrow, title, lead: null, aside: [], content } },
});
const table = (view: string) => ({
  type: {
    Table: {
      items_key: view,
      as: "r",
      headers: [{ label: "Team", key: "name" }],
      row: [text("{r.name}")],
      row_state: null,
    },
  },
});

function standings(): Json {
  return {
    id: "standings",
    name: "Standings",
    nodes: {
      main: {
        title: "Standings",
        visuals: [
          {
            id: "page",
            display: {
              Page: {
                width: "Normal",
                content: [
                  {
                    type: {
                      Hero: {
                        eyebrow: "NFL",
                        title: "Standings",
                        lead: "Every division.",
                        aside: [],
                        actions: [],
                      },
                    },
                  },
                  section(
                    "AFC seeds",
                    [table("sys.afc_picture")],
                    "Playoff picture",
                  ),
                  section(
                    "NFC seeds",
                    [table("sys.nfc_picture")],
                    "Playoff picture",
                  ),
                  section("AFC East", [table("sys.afc_east")]),
                  section("NFC West", [
                    table("sys.nfc_west"),
                    text("{sys_meta.attribution}", "Fine"),
                  ]),
                ],
              },
            },
          },
        ],
      },
    },
  };
}

const CONTENT: Path = [
  "nodes",
  "main",
  "visuals",
  0,
  "display",
  "Page",
  "content",
];
const at = (...rest: Path): Path => [...CONTENT, ...rest];
const titles = (def: Json) => pageTree(def).map((item) => item.label);

test("a page with one frame lists its blocks with their own names", () => {
  const items = pageTree(standings());
  assert.deepEqual(titles(standings()), [
    "Standings",
    "AFC seeds",
    "NFC seeds",
    "AFC East",
    "NFC West",
  ]);
  assert.equal(items[1].kind, "Section");
  assert.deepEqual(items[1].holds, at(1, "type", "Section", "content"));
  assert.equal(items[1].children[0].label, "Table of afc picture");
  assert.equal(
    items[1].children[0].children.length,
    0,
    "a table row is not something to arrange",
  );
  assert.equal(items[1].bound, false);
  assert.equal(items[1].children[0].bound, true);
});

test("the playoff picture moves to the bottom, and says so", () => {
  let def = standings();
  const moved = moveTo(def, at(2), CONTENT, 5)!;
  def = moved.def;
  const again = moveTo(def, at(1), CONTENT, 5)!;
  assert.deepEqual(titles(again.def), [
    "Standings",
    "AFC East",
    "NFC West",
    "NFC seeds",
    "AFC seeds",
  ]);
  assert.deepEqual(again.path, at(4));
  assert.equal(
    moveSummary(again.def, again.path),
    "Moved AFC seeds to the bottom of the page.",
  );
  assert.equal(
    moveSummary(moved.def, moved.path),
    "Moved NFC seeds to the bottom of the page.",
  );
});

test("moving down within the same list counts the gap it leaves", () => {
  const moved = moveTo(standings(), at(1), CONTENT, 3)!;
  assert.deepEqual(titles(moved.def), [
    "Standings",
    "NFC seeds",
    "AFC seeds",
    "AFC East",
    "NFC West",
  ]);
  assert.equal(
    moveSummary(moved.def, moved.path),
    "Moved AFC seeds below NFC seeds.",
  );
  assert.equal(
    moveTo(standings(), at(1), CONTENT, 1),
    null,
    "dropping a block where it is changes nothing",
  );
  assert.equal(moveTo(standings(), at(1), CONTENT, 2), null);
});

test("a block moves into a later section and the section keeps its place", () => {
  const into = at(3, "type", "Section", "content");
  const moved = moveTo(standings(), at(1), into, 0)!;
  assert.deepEqual(titles(moved.def), [
    "Standings",
    "NFC seeds",
    "AFC East",
    "NFC West",
  ]);
  assert.deepEqual(moved.path, at(2, "type", "Section", "content", 0));
  assert.equal(
    getAt(moved.def, [...moved.path, "type", "Section", "title"]),
    "AFC seeds",
  );
});

test("a block never moves inside itself", () => {
  assert.equal(
    moveTo(standings(), at(1), at(1, "type", "Section", "content"), 0),
    null,
  );
});

test("Alt arrows step through neighbours and out of a section at its ends", () => {
  const up = moveBy(standings(), at(2), -1)!;
  assert.deepEqual(titles(up.def), [
    "Standings",
    "NFC seeds",
    "AFC seeds",
    "AFC East",
    "NFC West",
  ]);
  const out = moveBy(standings(), at(4, "type", "Section", "content", 1), 1)!;
  assert.deepEqual(out.path, at(5));
  assert.equal(pageTree(out.def)[4].children.length, 1);
  assert.equal(
    moveBy(standings(), at(0), -1),
    null,
    "the first block of the page stays put",
  );
});

test("delete and duplicate", () => {
  const gone = removeAt(standings(), at(3));
  assert.deepEqual(titles(gone), [
    "Standings",
    "AFC seeds",
    "NFC seeds",
    "NFC West",
  ]);
  const twice = duplicateAt(standings(), at(1))!;
  assert.deepEqual(titles(twice.def).slice(1, 3), ["AFC seeds", "AFC seeds"]);
  assert.deepEqual(twice.path, at(2));
  assert.equal(titles(standings()).length, 5, "the original is left alone");
});

test("only plain words are offered for editing", () => {
  const def = standings();
  const hero = editableFields(getAt(def, at(0)));
  assert.deepEqual(
    hero.map((field) => field.field),
    ["eyebrow", "title", "lead"],
  );
  const credit = editableFields(
    getAt(def, at(4, "type", "Section", "content", 1)),
  );
  assert.deepEqual(credit, [], "a template is for the agent to change");
  assert.equal(
    isBound(getAt(def, at(4, "type", "Section", "content", 1))),
    true,
  );
  const renamed = setField(def, at(1), "title", "Seeds");
  assert.equal(pageTree(renamed)[1].label, "Seeds");
});

test("the preview finds each block by the object it draws", () => {
  const def = standings();
  const keys = blockKeys(def);
  const section = getAt(def, at(1)) as object;
  assert.equal(keys.get(section), at(1).join("/"));
});
