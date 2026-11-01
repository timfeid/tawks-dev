import * as THREE from 'three';

export type FocusSlot = 'head' | 'face' | 'neck' | 'torso' | 'hands' | 'legs' | 'feet';

/** Where on the character a wardrobe item is worn, for framing a preview of it. */
export function itemFocus(id: string): FocusSlot | null {
  if (id.startsWith('hat-')) return 'head';
  if (id === 'extra-bowtie' || id === 'extra-scarf' || id === 'extra-medal') return 'neck';
  if (['extra-santa-hat', 'extra-elf-hat', 'extra-antlers', 'extra-headphones', 'extra-halo'].includes(id)) return 'head';
  if (id.startsWith('extra-')) return 'face';
  if (id.startsWith('reaction-')) return 'face';
  if (id.startsWith('shirt-')) return 'torso';
  if (id.startsWith('gloves-')) return 'hands';
  if (id.startsWith('pants-')) return 'legs';
  if (id.startsWith('shoes-')) return 'feet';
  return null;
}

const boxOf = (root: THREE.Object3D, name: string) => {
  const part = root.getObjectByName(name);
  if (!part) return null;
  const box = new THREE.Box3().setFromObject(part);
  return box.isEmpty() ? null : box;
};

/** World-space bounds of the worn legs or feet garment alone, for a tight still of it. */
export function garmentBounds(character: THREE.Object3D, slot: FocusSlot): THREE.Box3 | null {
  const name = slot === 'legs' ? 'worn-pants' : slot === 'feet' ? 'worn-shoes' : null;
  if (!name) return null;
  character.updateMatrixWorld(true);
  const box = new THREE.Box3();
  character.traverse((part) => {
    if (part.name === name) box.expandByObject(part);
  });
  return box.isEmpty() ? null : box;
}

/** World-space bounds of the part a slot is worn on, with enough around it to read in context. */
export function focusBounds(character: THREE.Object3D, slot: FocusSlot): THREE.Box3 | null {
  character.updateMatrixWorld(true);
  const head = boxOf(character, 'character-head');
  const torso = boxOf(character, 'body-build-torso');
  const legs = boxOf(character, 'body-build-legs');
  switch (slot) {
    case 'head':
      // An imported model has no round head; its holiday hat rides on the model's head anchor.
      return head ?? boxOf(character, 'worn-holiday-head');
    case 'face': {
      const extras = boxOf(character, 'worn-head-extras');
      return extras && head ? extras.union(head) : (extras ?? head);
    }
    case 'neck': {
      const tie = boxOf(character, 'worn-neck-extras'), scarf = boxOf(character, 'worn-scarf');
      const worn = tie && scarf ? tie.union(scarf) : (tie ?? scarf);
      return worn?.expandByVector(new THREE.Vector3(0.35, 0.32, 0)) ?? null;
    }
    case 'torso':
    case 'hands':
      return torso;
    case 'legs': {
      if (!torso || !legs) return null;
      const box = torso.union(legs);
      box.max.y = box.min.y + (box.max.y - box.min.y) * 0.62;
      return box;
    }
    case 'feet':
      return legs?.expandByVector(new THREE.Vector3(0.05, 0.05, 0)) ?? null;
  }
}
