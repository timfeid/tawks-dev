import * as THREE from 'three';
import { CDN_BASE } from './cdn';

type ManaCostLike = {
  neon?: number;
  glitch?: number;
  chrome?: number;
  slag?: number;
  bio?: number;
  colorless?: number;
};

type CardLike = {
  name: string;
  colors?: string[] | null;
  image_url?: string | null;
  cost: ManaCostLike;
  kind_label?: string;
  description?: string;
  kind_tag?: string;
  playable?: boolean;
  can_cast?: boolean;
  can_play_land?: boolean;
  zone?: string | null;
  /** Pre-rendered rule-text lines (one per keyword/ability/effect) from
   * the backend. Each is one visual line in the card's rules box. */
  rule_lines?: string[];
  attached_to_name?: string | null;
  remembered_player_name?: string | null;
  owner_name?: string | null;
  stat_modifiers?: Array<{
    label: string;
    power_mod: number;
    toughness_mod: number;
    source_card_id?: string | null;
    source_card_name?: string | null;
    granted_keywords?: string[];
  }>;
};

/**
 * Draw a single rule entry (potentially word-wrapped) at (x, y) and
 * return the vertical height consumed in pixels. Respects optional
 * maxHeight; truncates with an ellipsis if needed.
 */
const wrapTextLines = (
  ctx: CanvasRenderingContext2D,
  text: string,
  x: number,
  y: number,
  maxWidth: number,
  lineHeight: number,
  maxHeight?: number,
): number => {
  const words = text.split(' ');
  let line = '';
  let ly = y;
  let truncated = false;

  for (let n = 0; n < words.length; n++) {
    const testLine = line + words[n] + ' ';
    const testWidth = ctx.measureText(testLine).width;

    if (maxHeight != null && (ly - y + lineHeight) > maxHeight) {
      truncated = true;
      break;
    }

    if (testWidth > maxWidth && n > 0) {
      ctx.fillText(line, x, ly);
      line = words[n] + ' ';
      ly += lineHeight;
    } else {
      line = testLine;
    }
  }

  if (truncated && line.length > 0) {
    ctx.fillText(line.trim() + '...', x, ly);
  } else if (line.length > 0) {
    ctx.fillText(line, x, ly);
  }

  return ly - y + lineHeight;
};

/**
 * The backend produces a pre-rendered list of rule-text lines for each
 * card (keywords with descriptions, then activated abilities, then any
 * static effect text). Frontend just reads that list.
 */
const buildRuleLines = (card: CardLike): string[] =>
  (card.rule_lines ?? []).filter((line) => line && line.trim().length > 0);

type TextureUtilsOptions = {
  cardTextureScale: number;
  cardW: number;
  cardH: number;
};

export const createTextureUtils = (options: TextureUtilsOptions) => {
  const { cardTextureScale, cardW, cardH } = options;
  const cardTextureCache = new Map<string, THREE.CanvasTexture>();
  const textureStatus = new Map<string, 'loading' | 'done'>();
  const buttonTextureCache = new Map<string, THREE.CanvasTexture>();
  const manaPalette = [
    { key: 'neon', color: '#00e5ff' },
    { key: 'glitch', color: '#a855f7' },
    { key: 'chrome', color: '#cbd5e1' },
    { key: 'slag', color: '#f97316' },
    { key: 'bio', color: '#22c55e' },
    { key: 'colorless', color: '#64748b' },
  ] as const;

  const toHex = (color: string) => `#${new THREE.Color(color).getHexString()}`;
  const lighten = (color: string, amount: number) =>
    `#${new THREE.Color(color).lerp(new THREE.Color(0xffffff), amount).getHexString()}`;

  const resolveManaStops = (card: CardLike) => {
    const provided = (card.colors || []).filter((color) => color);
    if (provided.length > 0) {
      const stops = provided.map((color) => toHex(color));
      return { primary: stops[0], stops };
    }
    const cost = card.cost ?? {};
    const raw = manaPalette
      .map((entry) => ({
        key: entry.key,
        color: entry.color,
        count: (cost as any)[entry.key] ?? 0,
      }))
      .filter((entry) => entry.count > 0);
    const colored = raw.filter((entry) => entry.key !== 'colorless');
    const effective = (colored.length ? colored : raw)
      .sort((a, b) => b.count - a.count)
      .map((entry) => toHex(entry.color));

    if (effective.length > 0) {
      return {
        primary: effective[0],
        stops: effective,
      };
    }

    const fallback = toHex('#f59e0b');
    return { primary: fallback, stops: [fallback] };
  };

  const createCardBackTexture = () => {
    const texture = new THREE.TextureLoader().load(`${CDN_BASE}/card_back.jpg`);
    texture.colorSpace = THREE.SRGBColorSpace;
    texture.minFilter = THREE.LinearFilter;
    texture.magFilter = THREE.LinearFilter;
    texture.generateMipmaps = false;
    texture.anisotropy = 1;
    return texture;
  };

  const createPileOutline = (color: string) => {
    const paint = document.createElement('canvas');
    paint.width = 400;
    paint.height = 600;
    const ctx = paint.getContext('2d');
    if (!ctx) return null;
    ctx.strokeStyle = color;
    ctx.shadowColor = color;
    ctx.shadowBlur = 20;
    ctx.lineWidth = 8;
    ctx.roundRect(10, 10, 380, 580, 40);
    ctx.stroke();
    const texture = new THREE.CanvasTexture(paint);
    texture.colorSpace = THREE.SRGBColorSpace;
    const material = new THREE.MeshBasicMaterial({
      map: texture,
      transparent: true,
      blending: THREE.AdditiveBlending,
      depthWrite: false,
    });
    const mesh = new THREE.Mesh(
      new THREE.PlaneGeometry(cardW, cardH),
      material,
    );
    mesh.rotation.x = -Math.PI / 2;
    return mesh;
  };

  const createCardTexture = (card: CardLike) => {
    const c = card as any;
    const ruleLinesKey = (card.rule_lines ?? []).join('|');
    const modsKey = ((card.stat_modifiers ?? []) as any[])
      .map(
        (m) =>
          `${m.label}:${m.power_mod}/${m.toughness_mod}:${(m.granted_keywords ?? []).join('+')}`,
      )
      .join(',');
    const catsKey = ((card as any).categories ?? []).join(',');
    const cacheKey = `${card.name}-${JSON.stringify(card.colors || [])}-${card.image_url}-${JSON.stringify(card.cost)}-${card.playable}-${card.zone}-${c.loyalty}-${c.power}-${c.toughness}-${c.counters || 0}-${c.damage || 0}-${ruleLinesKey}-${card.attached_to_name || ''}-${card.remembered_player_name || ''}-${card.owner_name || ''}-${modsKey}-${catsKey}`;
    if (cardTextureCache.has(cacheKey)) {
      return cardTextureCache.get(cacheKey)!;
    }

    const scale = cardTextureScale;
    const w = 300 * scale;
    const h = 440 * scale;
    const radius = 12 * scale;

    const { primary, stops } = resolveManaStops(card);
    const accent = new THREE.Color(primary);

    // Dark, sleek background
    const bgGradient = (ctx: CanvasRenderingContext2D, h: number) => {
      const g = ctx.createLinearGradient(0, 0, 0, h);
      g.addColorStop(0, '#020408');
      g.addColorStop(1, '#0f1729');
      return g;
    };

    const paint = document.createElement('canvas');
    paint.width = w;
    paint.height = h;
    const ctx = paint.getContext('2d');
    if (!ctx) return null;
    ctx.imageSmoothingEnabled = true;
    ctx.imageSmoothingQuality = 'high';

    const roundRect = (
      x: number,
      y: number,
      width: number,
      height: number,
      r: number,
    ) => {
      const clamped = Math.min(r, width / 2, height / 2);
      ctx.beginPath();
      ctx.moveTo(x + clamped, y);
      ctx.arcTo(x + width, y, x + width, y + height, clamped);
      ctx.arcTo(x + width, y + height, x, y + height, clamped);
      ctx.arcTo(x, y + height, x, y, clamped);
      ctx.arcTo(x, y, x + width, y, clamped);
      ctx.closePath();
    };

    const drawCard = (image?: HTMLImageElement) => {
      ctx.clearRect(0, 0, w, h);

      // 1. Base Card Shape (Background)
      ctx.fillStyle = bgGradient(ctx, h);
      roundRect(0, 0, w, h, radius);
      ctx.fill();

      // 2. Art Layer (Perfect Square)
      const artH = w;
      ctx.save();
      roundRect(0, 0, w, h, radius);
      ctx.clip();

      if (image) {
        // Center crop the image
        const imgScale = Math.max(w / image.width, artH / image.height);
        const drawW = image.width * imgScale;
        const drawH = image.height * imgScale;
        const drawX = (w - drawW) / 2;
        const drawY = (artH - drawH) / 2; // Center vertically as well
        ctx.drawImage(image, drawX, drawY, drawW, drawH);
      } else {
        const artGrad = ctx.createLinearGradient(0, 0, 0, artH);
        artGrad.addColorStop(0, '#1e293b');
        artGrad.addColorStop(1, '#0f172a');
        ctx.fillStyle = artGrad;
        ctx.fillRect(0, 0, w, artH);

        // Placeholder pattern
        ctx.strokeStyle = 'rgba(255,255,255,0.05)';
        ctx.lineWidth = 2 * scale;
        ctx.beginPath();
        for (let i = 0; i < w; i += 20 * scale) {
          ctx.moveTo(i, 0);
          ctx.lineTo(i - artH, artH);
        }
        ctx.stroke();
      }

      // Gradient fade at bottom of art
      const fadeH = 60 * scale;
      const fade = ctx.createLinearGradient(0, artH - fadeH, 0, artH);
      fade.addColorStop(0, 'rgba(2, 4, 8, 0)');
      fade.addColorStop(1, 'rgba(2, 4, 8, 1)');
      ctx.fillStyle = fade;
      ctx.fillRect(0, artH - fadeH, w, fadeH);
      ctx.restore();

      // 3. Frame / Border
      ctx.save();
      ctx.strokeStyle = primary;
      ctx.lineWidth = 2 * scale;
      ctx.shadowColor = primary;
      ctx.shadowBlur = 8 * scale;
      roundRect(2 * scale, 2 * scale, w - 4 * scale, h - 4 * scale, radius);
      ctx.stroke();
      ctx.restore();

      // 4. Text Content Area
      const contentY = artH;
      const contentH = h - contentY;

      // Title Bar
      const titleBarH = 32 * scale;
      const titleBarY = contentY - titleBarH / 2; // Overlap art slightly

      ctx.fillStyle = 'rgba(15, 23, 42, 0.95)';
      ctx.shadowColor = 'rgba(0,0,0,0.8)';
      ctx.shadowBlur = 10 * scale;
      ctx.beginPath();
      ctx.roundRect(
        10 * scale,
        titleBarY,
        w - 20 * scale,
        titleBarH,
        4 * scale,
      );
      ctx.fill();
      ctx.shadowBlur = 0;

      // Mana Cost calculation (to prevent name overlap)
      const cost = card.cost ?? {};
      const pipCount =
        (cost.bio || 0) +
        (cost.slag || 0) +
        (cost.chrome || 0) +
        (cost.glitch || 0) +
        (cost.neon || 0) +
        ((cost.colorless || 0) > 0 ? 1 : 0);
      const spacing = 13 * scale;
      const manaCostWidth = pipCount > 0 ? (pipCount * spacing) + 10 * scale : 0;

      const textInset = 16 * scale;

      // Card Name
      ctx.fillStyle = '#ffffff';
      ctx.font = `700 ${14 * scale}px "Orbitron", sans-serif`;
      ctx.textAlign = 'left';
      ctx.textBaseline = 'middle';
      const nameX = textInset;
      const nameMaxWidth = w - nameX - manaCostWidth - 10 * scale;
      ctx.fillText(
        (card.name || '').toUpperCase(),
        nameX,
        titleBarY + titleBarH / 2 + 1 * scale,
        nameMaxWidth
      );

      // Mana Cost (Top Right of Title Bar)
      const hasCost = pipCount > 0;

      if (hasCost) {
        let pipX = w - 24 * scale;
        const pipY = titleBarY + titleBarH / 2;
        const pipR = 5.5 * scale;

        const drawPip = (color: string, count: number, isColorless = false) => {
          for (let i = 0; i < count; i++) {
            ctx.beginPath();
            // Hexagon shape
            for (let j = 0; j < 6; j++) {
              const angle = (j * 2 * Math.PI) / 6 - Math.PI / 2;
              const px = pipX + Math.cos(angle) * pipR;
              const py = pipY + Math.sin(angle) * pipR;
              if (j === 0) ctx.moveTo(px, py);
              else ctx.lineTo(px, py);
            }
            ctx.closePath();

            ctx.fillStyle = color;
            ctx.fill();
            if (isColorless) {
              ctx.fillStyle = '#fff';
              ctx.font = `bold ${8 * scale}px sans-serif`;
              ctx.textAlign = 'center';
              ctx.textBaseline = 'middle';
              ctx.fillText(count.toString(), pipX, pipY);
              // colorless usually grouped, so we break after drawing one with number
              i = count;
            } else {
              // Gloss
              ctx.beginPath();
              ctx.arc(
                pipX - pipR * 0.3,
                pipY - pipR * 0.3,
                pipR * 0.3,
                0,
                Math.PI * 2,
              );
              ctx.fillStyle = 'rgba(255,255,255,0.4)';
              ctx.fill();
            }
            pipX -= spacing;
          }
        };

        const bio = cost.bio ?? 0;
        const slag = cost.slag ?? 0;
        const chrome = cost.chrome ?? 0;
        const glitch = cost.glitch ?? 0;
        const neon = cost.neon ?? 0;
        const colorless = cost.colorless ?? 0;

        if (bio > 0) drawPip('#22c55e', bio);
        if (slag > 0) drawPip('#f97316', slag);
        if (chrome > 0) drawPip('#cbd5e1', chrome);
        if (glitch > 0) drawPip('#a855f7', glitch);
        if (neon > 0) drawPip('#00e5ff', neon);
        if (colorless > 0) drawPip('#64748b', colorless, true);
      }

      // Type Line + Categories
      const typeY = titleBarY + titleBarH + 6 * scale;
      ctx.fillStyle = primary;
      ctx.font = `600 ${9 * scale}px "Orbitron", sans-serif`;
      ctx.textAlign = 'left';
      const kindLabel = (card.kind_label || card.kind_tag || 'UNKNOWN').toUpperCase();
      const categories = ((card as any).categories ?? []) as string[];
      const catSuffix =
        categories.length > 0
          ? ` · ${categories.map((c: string) => c.replace(/_/g, ' ').toUpperCase()).join(' · ')}`
          : '';
      ctx.fillText(
        kindLabel + catSuffix,
        textInset,
        typeY + 10 * scale,
        w - textInset * 2,
      );

      // Relationship info — rendered ABOVE rules text so it's never
      // clipped by the P/T badge reserve at the bottom.
      let preRulesY = typeY + 18 * scale;
      const preLineH = 10 * scale;
      // Attached-to is visually obvious from card positioning in the
      // scene (overlays its target), so we skip rendering it on the face.
      //
      // Remembered player: show just "TARGET → NAME" — the full "Choose
      // target player" label already appears in the rule text so we
      // don't repeat it.
      if (card.remembered_player_name) {
        // Render as a rules-text line: ability text on the left in the
        // same style as rule lines, chosen name on the right in cyan.
        const fontSize = card.kind_tag === 'deck' ? 9 : 10;
        ctx.font = `400 ${fontSize * scale}px "Roboto", sans-serif`;
        ctx.fillStyle = '#e2e8f0'; // same as rules text color
        ctx.textAlign = 'left';
        ctx.textBaseline = 'top';
        ctx.fillText('Choose target player', textInset, preRulesY);
        ctx.fillStyle = '#67e8f9'; // cyan for the chosen name
        ctx.textAlign = 'right';
        ctx.fillText(
          card.remembered_player_name.toUpperCase(),
          w - textInset,
          preRulesY,
        );
        ctx.textAlign = 'left';
        preRulesY += preLineH;
      }

      // Rules Text
      const rulesY = preRulesY;
      const rulesW = w - textInset * 2;

      ctx.fillStyle = '#e2e8f0';

      // Use smaller font for deck cards to fit longer descriptions
      const isDeckCard = card.kind_tag === 'deck';
      const fontSize = isDeckCard ? 9 : 10;
      const lineHeightMultiplier = isDeckCard ? 12 : 14;

      ctx.font = `400 ${fontSize * scale}px "Roboto", sans-serif`;
      ctx.textAlign = 'left';
      ctx.textBaseline = 'top';

      const wrapText = (
        text: string,
        x: number,
        y: number,
        maxWidth: number,
        lineHeight: number,
        maxHeight?: number,
      ) => {
        const words = text.split(' ');
        let line = '';
        let ly = y;
        let truncated = false;

        for (let n = 0; n < words.length; n++) {
          const testLine = line + words[n] + ' ';
          const metrics = ctx.measureText(testLine);
          const testWidth = metrics.width;

          // Check if we've exceeded max height
          if (maxHeight && (ly - y + lineHeight) > maxHeight) {
            truncated = true;
            break;
          }

          if (testWidth > maxWidth && n > 0) {
            ctx.fillText(line, x, ly);
            line = words[n] + ' ';
            ly += lineHeight;
          } else {
            line = testLine;
          }
        }

        // If truncated, add ellipsis to last line
        if (truncated && line.length > 0) {
          const ellipsisLine = line.trim() + '...';
          ctx.fillText(ellipsisLine, x, ly);
        } else if (line.length > 0) {
          ctx.fillText(line, x, ly);
        }
      };

      const maxHeight = isDeckCard ? 480 * scale : undefined;
      const lineH = lineHeightMultiplier * scale;

      // Filter out "Choose target player" from rule lines when we're
      // already rendering it with the annotated player name above.
      const rawRuleLines = buildRuleLines(card);
      const ruleLines = card.remembered_player_name
        ? rawRuleLines.filter(
            (line) => !/choose\s+(a\s+)?target\s+player/i.test(line),
          )
        : rawRuleLines;
      let rulesEndY = rulesY;
      if (ruleLines.length === 0) {
        const fallback =
          card.description ||
          (card.kind_tag === 'unit' ? 'No abilities.' : 'Effect unknown.');
        wrapText(fallback, textInset, rulesY, rulesW, lineH, maxHeight);
        rulesEndY = rulesY + lineH;
      } else {
        // Render each rule on its own visual line (with wrapping inside).
        // Track vertical progress and leave a small gap between entries.
        let ly = rulesY;
        const interLineGap = 3 * scale;
        for (const entry of ruleLines) {
          if (maxHeight && ly - rulesY > maxHeight) break;
          const remaining = maxHeight != null ? maxHeight - (ly - rulesY) : undefined;
          const consumed = wrapTextLines(
            ctx,
            entry,
            textInset,
            ly,
            rulesW,
            lineH,
            remaining,
          );
          ly += consumed + interLineGap;
        }
        rulesEndY = ly;
      }

      // ── Extended detail block: attachment / chosen player / owner /
      // stat modifier sources. Rendered below the rule text so the card
      // itself carries all the info the old inspect HUD showed.
      let infoY = rulesEndY + 4 * scale;
      const infoLineH = 11 * scale;
      const reserveForBadges = 44 * scale; // P/T + counter badges at bottom
      const infoMaxY = h - reserveForBadges;

      const writeInfo = (
        text: string,
        color: string,
        font = `600 ${8 * scale}px "Orbitron", sans-serif`,
      ) => {
        if (infoY + infoLineH > infoMaxY) return;
        ctx.font = font;
        ctx.fillStyle = color;
        ctx.textAlign = 'left';
        ctx.textBaseline = 'top';
        const trimmed = text.length > 48 ? text.slice(0, 47) + '…' : text;
        ctx.fillText(trimmed, textInset, infoY);
        infoY += infoLineH;
      };

      // Relationship info (attached_to, chosen player, owner) now
      // renders ABOVE the rules text — see drawPreLine block. Only stat
      // modifiers remain in this post-rules section.
      const mods = (card.stat_modifiers ?? []).filter(
        (m) =>
          m.power_mod !== 0 ||
          m.toughness_mod !== 0 ||
          (m.granted_keywords?.length ?? 0) > 0,
      );
      if (mods.length > 0) {
        if (infoY + infoLineH <= infoMaxY) {
          ctx.font = `600 ${7 * scale}px "Orbitron", sans-serif`;
          ctx.fillStyle = '#64748b';
          ctx.fillText('MODIFIERS', textInset, infoY);
          infoY += infoLineH;
        }
        for (const m of mods) {
          if (infoY + infoLineH > infoMaxY) break;
          const fmt = (n: number) => (n >= 0 ? `+${n}` : `${n}`);
          const grants = m.granted_keywords ?? [];
          const hasStats = m.power_mod !== 0 || m.toughness_mod !== 0;
          const positive = m.power_mod > 0 || m.toughness_mod > 0;
          const label = m.label.toUpperCase();
          ctx.font = `600 ${9 * scale}px "Orbitron", sans-serif`;
          ctx.fillStyle = '#cbd5e1';
          ctx.textAlign = 'left';
          ctx.fillText(
            label.length > 28 ? label.slice(0, 27) + '…' : label,
            textInset,
            infoY,
          );
          const parts: Array<{ text: string; color: string }> = [];
          if (grants.length > 0) {
            parts.push({ text: grants.join(', '), color: '#7dd3fc' });
          }
          if (hasStats) {
            parts.push({
              text: `${fmt(m.power_mod)}/${fmt(m.toughness_mod)}`,
              color: positive ? '#86efac' : '#fca5a5',
            });
          }
          let rightX = w - textInset;
          ctx.textAlign = 'right';
          for (let i = parts.length - 1; i >= 0; i--) {
            const part = parts[i];
            ctx.fillStyle = part.color;
            ctx.fillText(part.text, rightX, infoY);
            rightX -= ctx.measureText(part.text).width + 6 * scale;
          }
          ctx.textAlign = 'left';
          infoY += infoLineH;
        }
      }

      // Power/Toughness (if unit) — same single-pill billboard rendering as
      // the floating top-of-card billboard, embedded directly on the card
      // face so the two look identical and can be compared side-by-side.
      const c = card as any;
      if (c.power != null && c.toughness != null) {
        const pBase = c.base_power ?? c.power;
        const tBase = c.base_toughness ?? c.toughness;
        const dmg = c.damage ?? 0;
        const neutralColor = '#94a3b8';
        const powerColor =
          c.power > pBase ? '#4ade80' : c.power < pBase ? '#f87171' : neutralColor;
        const toughColor =
          c.toughness > tBase
            ? '#4ade80'
            : c.toughness < tBase || dmg > 0
              ? '#f87171'
              : neutralColor;
        const isModified = c.power !== pBase || c.toughness !== tBase || dmg > 0;
        const borderColor = isModified ? '#f59e0b' : '#475569';

        const remaining = Math.max(0, c.toughness - dmg);
        const toughText = `${dmg > 0 ? remaining : c.toughness}`;

        const ptBoxW = 90 * scale;
        const ptBoxH = 32 * scale;
        const ptX = w - ptBoxW - 12 * scale;
        const ptY = h - ptBoxH - 12 * scale;
        drawPTPill(
          ctx,
          ptX,
          ptY,
          ptBoxW,
          ptBoxH,
          c.power,
          toughText,
          powerColor,
          toughColor,
          borderColor,
          ptBoxH,
        );
      }

      // Loyalty (if Agent)
      if (c.loyalty != null) {
        const ptBoxW = 40 * scale;
        const ptBoxH = 28 * scale;
        const ptX = w - ptBoxW - 12 * scale;
        const ptY = h - ptBoxH - 12 * scale;

        ctx.fillStyle = '#475569'; // Slate-600
        ctx.beginPath();
        // Hexagon shape for loyalty
        for (let j = 0; j < 6; j++) {
          const angle = (j * 2 * Math.PI) / 6 - Math.PI / 2;
          const px = ptX + ptBoxW / 2 + Math.cos(angle) * (ptBoxW / 2);
          const py = ptY + ptBoxH / 2 + Math.sin(angle) * (ptBoxH / 2);
          if (j === 0) ctx.moveTo(px, py);
          else ctx.lineTo(px, py);
        }
        ctx.closePath();
        ctx.fill();

        ctx.strokeStyle = '#fff';
        ctx.lineWidth = 1 * scale;
        ctx.stroke();

        ctx.fillStyle = '#fff';
        ctx.font = `800 ${14 * scale}px "Orbitron", sans-serif`;
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText(
          `${c.loyalty}`,
          ptX + ptBoxW / 2,
          ptY + ptBoxH / 2 + 1 * scale,
        );
      }

      // Counters Badge (bottom left, opposite P/T box)
      const counters = c.counters ?? 0;
      if (counters > 0) {
        const cBadgeW = 54 * scale;
        const cBadgeH = 28 * scale;
        const cBadgeX = 12 * scale;
        const cBadgeY = h - cBadgeH - 12 * scale;

        ctx.fillStyle = '#1e293b';
        ctx.beginPath();
        ctx.roundRect(cBadgeX, cBadgeY, cBadgeW, cBadgeH, 6 * scale);
        ctx.fill();

        ctx.strokeStyle = '#22c55e';
        ctx.lineWidth = 1.5 * scale;
        ctx.shadowColor = '#22c55e';
        ctx.shadowBlur = 6 * scale;
        ctx.stroke();
        ctx.shadowBlur = 0;

        ctx.fillStyle = '#22c55e';
        ctx.font = `800 ${12 * scale}px "Orbitron", sans-serif`;
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText(
          `+${counters}`,
          cBadgeX + cBadgeW / 2,
          cBadgeY + cBadgeH / 2 + 1 * scale,
        );
      }

      // Owner name — bottom-left, just the name, no label prefix.
      if (card.owner_name) {
        ctx.save();
        ctx.font = `700 ${8 * scale}px "Orbitron", sans-serif`;
        ctx.fillStyle = '#94a3b8';
        ctx.textAlign = 'left';
        ctx.textBaseline = 'bottom';
        ctx.shadowColor = 'rgba(0,0,0,0.8)';
        ctx.shadowBlur = 4 * scale;
        ctx.fillText(
          card.owner_name.toUpperCase(),
          textInset,
          h - 14 * scale,
        );
        ctx.restore();
      }

      const isGraveyardCastable = card.zone === 'graveyard' && card.can_cast;

      // Playable Badge
      if (card.playable) {
        const badgeW = 140 * scale;
        const badgeH = 28 * scale;
        const badgeX = (w - badgeW) / 2;
        const badgeY = 12 * scale;

        const badgeFill = isGraveyardCastable ? '#a855f7' : '#06b6d4';
        const badgeText = isGraveyardCastable ? 'GRAVECAST' : 'PLAYABLE';
        const badgeTextColor = isGraveyardCastable ? '#f8fafc' : '#000000';

        ctx.fillStyle = badgeFill;
        ctx.shadowColor = badgeFill;
        ctx.shadowBlur = 20 * scale;
        ctx.beginPath();
        ctx.roundRect(badgeX, badgeY, badgeW, badgeH, 6 * scale);
        ctx.fill();
        ctx.shadowBlur = 0;

        ctx.fillStyle = badgeTextColor;
        ctx.font = `900 ${14 * scale}px "Orbitron", sans-serif`;
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText(
          badgeText,
          badgeX + badgeW / 2,
          badgeY + badgeH / 2 + 1 * scale,
        );

        if (isGraveyardCastable) {
          ctx.strokeStyle = 'rgba(255,255,255,0.85)';
          ctx.lineWidth = 1.5 * scale;
          ctx.beginPath();
          ctx.roundRect(
            badgeX + 1.5 * scale,
            badgeY + 1.5 * scale,
            badgeW - 3 * scale,
            badgeH - 3 * scale,
            5 * scale,
          );
          ctx.stroke();
        }
      }
    };

    const texture = new THREE.CanvasTexture(paint);
    texture.colorSpace = THREE.SRGBColorSpace;
    texture.minFilter = THREE.LinearMipmapLinearFilter;
    texture.magFilter = THREE.LinearFilter;
    texture.generateMipmaps = true;
    texture.anisotropy = 8;

    cardTextureCache.set(cacheKey, texture);
    drawCard();
    texture.needsUpdate = true;

    if (card.image_url) {
      textureStatus.set(cacheKey, 'loading');
      const artImage = new Image();
      artImage.crossOrigin = 'anonymous';
      artImage.onload = () => {
        drawCard(artImage);
        texture.needsUpdate = true;
        textureStatus.set(cacheKey, 'done');
      };
      artImage.src = card.image_url;
    } else {
      textureStatus.set(cacheKey, 'done');
    }

    return texture;
  };

  const createStatSprite = (label: string, color: string) => {
    const canvas = document.createElement('canvas');
    canvas.width = 256;
    canvas.height = 128;
    const ctx = canvas.getContext('2d');
    if (!ctx) return null;

    const r = 22;
    const roundRect = (
      x: number,
      y: number,
      width: number,
      height: number,
      radius: number,
    ) => {
      const clamped = Math.min(radius, width / 2, height / 2);
      ctx.beginPath();
      ctx.moveTo(x + clamped, y);
      ctx.arcTo(x + width, y, x + width, y + height, clamped);
      ctx.arcTo(x + width, y + height, x, y + height, clamped);
      ctx.arcTo(x, y + height, x, y, clamped);
      ctx.arcTo(x, y, x + width, y, clamped);
      ctx.closePath();
    };

    ctx.clearRect(0, 0, canvas.width, canvas.height);

    ctx.globalAlpha = 0.9;
    ctx.lineWidth = 4;
    ctx.strokeStyle = color;
    ctx.shadowColor = color;
    ctx.shadowBlur = 12;
    roundRect(10, 12, canvas.width - 20, canvas.height - 24, r);
    ctx.stroke();
    ctx.shadowBlur = 0;
    ctx.globalAlpha = 0.55;
    ctx.lineWidth = 2;
    ctx.strokeStyle = 'rgba(255,255,255,0.7)';
    roundRect(10, 12, canvas.width - 20, canvas.height - 24, r);
    ctx.stroke();
    ctx.globalAlpha = 1;
    ctx.fillStyle = '#0b111a';
    roundRect(10, 12, canvas.width - 20, canvas.height - 24, r);
    ctx.fill();
    ctx.strokeStyle = color;
    ctx.lineWidth = 3;
    roundRect(10, 12, canvas.width - 20, canvas.height - 24, r);
    ctx.stroke();
    ctx.fillStyle = '#f8fafc';
    ctx.font = '700 44px "Courier New", monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(label, canvas.width / 2, canvas.height / 2 + 4);

    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    const material = new THREE.SpriteMaterial({
      map: texture,
      transparent: true,
    });
    const sprite = new THREE.Sprite(material);
    sprite.scale.set(0.32, 0.16, 1);
    return sprite;
  };

  const createStatBarBillboard = (
    name: string,
    life: number,
    manaPool: any,
    _totalInfrastructures: number,
    operatorDamage?: Record<string, number>,
  ) => {
    const canvas = document.createElement('canvas');
    canvas.width = 320;
    canvas.height = 180;
    const ctx = canvas.getContext('2d');
    if (!ctx) return null;

    const healthColor = '#f43f5e';
    const darkBg = 'rgba(5, 10, 16, 0.95)';

    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.fillStyle = darkBg;
    ctx.roundRect(0, 0, canvas.width, canvas.height, 16);
    ctx.fill();
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.15)';
    ctx.lineWidth = 2;
    ctx.stroke();

    ctx.fillStyle = '#ffffff';
    ctx.font = '900 22px "Orbitron", sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText(name.toUpperCase(), 160, 32);
    ctx.fillStyle = 'rgba(255, 255, 255, 0.1)';
    ctx.fillRect(20, 42, 280, 2);

    const drawHeart = (x: number, y: number, w: number, h: number) => {
      ctx.beginPath();
      ctx.moveTo(x, y + h / 4);
      ctx.quadraticCurveTo(x, y, x + w / 4, y);
      ctx.quadraticCurveTo(x + w / 2, y, x + w / 2, y + h / 4);
      ctx.quadraticCurveTo(x + w / 2, y, x + (3 * w) / 4, y);
      ctx.quadraticCurveTo(x + w, y + h / 4, x + w, y + h / 4);
      ctx.quadraticCurveTo(x + w, y + h / 2, x + w / 2, y + h);
      ctx.quadraticCurveTo(x, y + h / 2, x, y + h / 4);
      ctx.closePath();
      ctx.fill();
    };

    ctx.fillStyle = healthColor;
    drawHeart(24, 60, 32, 30);
    ctx.fillStyle = '#ffffff';
    ctx.font = '700 44px "Orbitron", sans-serif';
    ctx.textAlign = 'right';
    ctx.fillText(`${life}`, 296, 88);

    // Operator Damage Display
    if (operatorDamage && Object.keys(operatorDamage).length > 0) {
      const totalDamage = Object.values(operatorDamage).reduce((sum, dmg) => sum + dmg, 0);
      const maxDamage = Math.max(...Object.values(operatorDamage));

      if (maxDamage > 0) {
        // Draw sword icon
        ctx.fillStyle = '#fbbf24';
        ctx.font = '700 20px "Orbitron", sans-serif';
        ctx.textAlign = 'left';
        ctx.fillText('⚔', 70, 88);

        // Show max damage (closest to lethal)
        ctx.fillStyle = maxDamage >= 15 ? '#ef4444' : '#fbbf24';
        ctx.font = '700 24px "Orbitron", sans-serif';
        ctx.fillText(`${maxDamage}/21`, 95, 88);
      }
    }

    const manaTypes = [
      { key: 'neon', color: '#00e5ff' },
      { key: 'glitch', color: '#a855f7' },
      { key: 'chrome', color: '#cbd5e1' },
      { key: 'slag', color: '#f97316' },
      { key: 'bio', color: '#22c55e' },
      { key: 'colorless', color: '#64748b' },
    ];

    let xOffset = 24;
    manaTypes.forEach((m) => {
      const val = manaPool[m.key];
      if (val > 0 || m.key === 'colorless') {
        ctx.fillStyle = m.color;
        ctx.beginPath();
        ctx.arc(xOffset + 10, 135, 8, 0, Math.PI * 2);
        ctx.fill();
        ctx.fillStyle = '#ffffff';
        ctx.font = '700 20px "Orbitron", sans-serif';
        ctx.textAlign = 'left';
        ctx.fillText(`${val}`, xOffset + 22, 142);
        xOffset += 45;
      }
    });

    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    const material = new THREE.SpriteMaterial({
      map: texture,
      transparent: true,
      depthTest: false,
      depthWrite: false,
    });
    const sprite = new THREE.Sprite(material);
    sprite.scale.set(1.0, 0.56, 1);
    sprite.renderOrder = 100;
    return sprite;
  };

  const create3DButton = (
    label: string,
    actionId: string,
    color: string = '#f59e0b',
    options?: {
      width?: number;
      height?: number;
      fontSize?: number;
      padX?: number;
      padY?: number;
      radius?: number;
      pixelRatio?: number;
    },
  ) => {
    const buttonW = options?.width ?? 2.8;
    const buttonH = options?.height ?? 0.9;
    const sizeKey = `${buttonW.toFixed(2)}x${buttonH.toFixed(2)}`;
    const cacheKey = `btn:${label}-${color}-${sizeKey}-touch-v3`;

    if (buttonTextureCache.has(cacheKey)) {
      const cachedTexture = buttonTextureCache.get(cacheKey)!;
      const geometry = new THREE.PlaneGeometry(buttonW, buttonH);
      const material = new THREE.MeshBasicMaterial({
        map: cachedTexture,
        transparent: true,
        side: THREE.DoubleSide,
        depthWrite: false,
        depthTest: true,
        color: 0xffffff,
        opacity: 0.95,
        alphaTest: 0.0,
      });
      const mesh = new THREE.Mesh(geometry, material);
      mesh.layers.set(1);
      mesh.renderOrder = 20;
      mesh.userData.isButton = true;
      mesh.userData.actionId = actionId;
      mesh.userData.baseScale = 1.0;
      mesh.userData.baseEmissiveIntensity = 0;
      return mesh;
    }

    const canvas = document.createElement('canvas');
    const pixelRatio = options?.pixelRatio ?? 2;
    const logicalW = Math.max(220, Math.round(600 * (buttonW / 2.8)));
    const logicalH = Math.max(120, Math.round(200 * (buttonH / 0.9)));
    canvas.width = Math.round(logicalW * pixelRatio);
    canvas.height = Math.round(logicalH * pixelRatio);
    const ctx = canvas.getContext('2d');
    if (!ctx) return null;
    ctx.scale(pixelRatio, pixelRatio);

    const padX = options?.padX ?? Math.round(logicalW * 0.06);
    const padY = options?.padY ?? Math.round(logicalH * 0.14);
    const drawW = logicalW - padX * 2;
    const drawH = logicalH - padY * 2;
    const radius = options?.radius ?? Math.round(Math.min(drawW, drawH) * 0.28);
    const fontSize = options?.fontSize ?? Math.round(drawH * 0.62);
    const borderW = Math.max(2, Math.round(drawH * 0.08));
    const borderW2 = Math.max(1, Math.round(drawH * 0.03));

    const drawRounded = () => {
      ctx.beginPath();
      ctx.moveTo(padX + radius, padY);
      ctx.lineTo(padX + drawW - radius, padY);
      ctx.quadraticCurveTo(padX + drawW, padY, padX + drawW, padY + radius);
      ctx.lineTo(padX + drawW, padY + drawH - radius);
      ctx.quadraticCurveTo(
        padX + drawW,
        padY + drawH,
        padX + drawW - radius,
        padY + drawH,
      );
      ctx.lineTo(padX + radius, padY + drawH);
      ctx.quadraticCurveTo(padX, padY + drawH, padX, padY + drawH - radius);
      ctx.lineTo(padX, padY + radius);
      ctx.quadraticCurveTo(padX, padY, padX + radius, padY);
      ctx.closePath();
    };

    ctx.clearRect(0, 0, canvas.width, canvas.height);

    ctx.fillStyle = '#e2f5ff';
    ctx.font = `800 ${fontSize}px "Orbitron", sans-serif`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.shadowColor = color;
    ctx.shadowBlur = 16;
    ctx.fillText(label.toUpperCase(), padX + drawW / 2, padY + drawH / 2);
    ctx.shadowBlur = 0;

    ctx.globalAlpha = 0.55;
    ctx.strokeStyle = color;
    ctx.lineWidth = 2;
    ctx.strokeText(label.toUpperCase(), padX + drawW / 2, padY + drawH / 2);
    ctx.globalAlpha = 1;

    // Border last so it stays crisp above glow
    ctx.globalAlpha = 0.95;
    ctx.lineWidth = borderW;
    ctx.strokeStyle = color;
    ctx.shadowColor = color;
    ctx.shadowBlur = 18;
    drawRounded();
    ctx.stroke();
    ctx.shadowBlur = 0;
    ctx.globalAlpha = 0.6;
    ctx.lineWidth = borderW2;
    ctx.strokeStyle = 'rgba(255,255,255,0.8)';
    drawRounded();
    ctx.stroke();
    ctx.globalAlpha = 1;

    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    texture.generateMipmaps = false;
    texture.minFilter = THREE.LinearFilter;
    texture.magFilter = THREE.LinearFilter;
    buttonTextureCache.set(cacheKey, texture);

    const geometry = new THREE.PlaneGeometry(buttonW, buttonH);
    const material = new THREE.MeshBasicMaterial({
      map: texture,
      transparent: true,
      side: THREE.DoubleSide,
      depthWrite: false,
      depthTest: true,
      color: 0xffffff,
      opacity: 0.95,
      alphaTest: 0.0,
    });

    const mesh = new THREE.Mesh(geometry, material);
    mesh.layers.set(1);
    mesh.renderOrder = 20;
    mesh.userData.isButton = true;
    mesh.userData.actionId = actionId;
    mesh.userData.baseScale = 1.0;
    mesh.userData.baseEmissiveIntensity = 0;
    return mesh;
  };

  const create3DTextButton = (
    label: string,
    actionId: string,
    color: string = '#f59e0b',
    active: boolean = false,
    options?: {
      subtitle?: string;
      fontSize?: number;
      pixelRatio?: number;
    },
  ) => {
    const text = label.toUpperCase();
    const subtitle = options?.subtitle?.toUpperCase();
    const pixelRatio = options?.pixelRatio ?? 2;
    const fontSize = options?.fontSize ?? 36;
    const subFontSize = Math.round(fontSize * 0.65);
    const cacheKey = `txt-btn:${text}-${subtitle || ''}-${color}-${active}-${fontSize}`;

    if (buttonTextureCache.has(cacheKey)) {
      const cachedTexture = buttonTextureCache.get(cacheKey)!;
      const aspect = cachedTexture.userData.aspect ?? 2;
      const height = 0.45; // Increased height for multi-line
      const width = height * aspect;
      const geometry = new THREE.PlaneGeometry(width, height);
      const material = new THREE.MeshBasicMaterial({
        map: cachedTexture,
        transparent: true,
        side: THREE.DoubleSide,
        depthWrite: false,
        depthTest: true,
        color: 0xffffff,
      });
      material.userData.baseColor = new THREE.Color(0xffffff);
      const mesh = new THREE.Mesh(geometry, material);
      mesh.layers.set(1);
      mesh.renderOrder = 22;
      mesh.userData.isButton = true;
      mesh.userData.actionId = actionId;
      return mesh;
    }

    // Measure text
    const tempCanvas = document.createElement('canvas');
    const tempCtx = tempCanvas.getContext('2d')!;
    tempCtx.font = `800 ${fontSize}px "Orbitron", sans-serif`;
    const metrics = tempCtx.measureText(text);
    let textWidth = metrics.width;
    if (subtitle) {
      tempCtx.font = `700 ${subFontSize}px "Orbitron", sans-serif`;
      textWidth = Math.max(textWidth, tempCtx.measureText(subtitle).width);
    }

    const paddingX = 20;
    const paddingY = 15;
    const lineGap = 8;
    const logicalW = textWidth + paddingX * 2;
    const logicalH = fontSize + (subtitle ? subFontSize + lineGap : 0) + paddingY * 2;

    const canvas = document.createElement('canvas');
    canvas.width = Math.round(logicalW * pixelRatio);
    canvas.height = Math.round(logicalH * pixelRatio);
    const ctx = canvas.getContext('2d');
    if (!ctx) return null;
    ctx.scale(pixelRatio, pixelRatio);

    ctx.clearRect(0, 0, logicalW, logicalH);
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';

    const centerY = logicalH / 2;
    const mainTextY = subtitle ? centerY - subFontSize / 2 - lineGap / 2 : centerY;

    ctx.font = `800 ${fontSize}px "Orbitron", sans-serif`;
    if (active) {
      ctx.fillStyle = color;
      ctx.shadowColor = color;
      ctx.shadowBlur = 10;
      ctx.fillText(text, logicalW / 2, mainTextY);
      ctx.shadowBlur = 0;
      ctx.strokeStyle = 'rgba(255,255,255,0.8)';
      ctx.lineWidth = 1;
      ctx.strokeText(text, logicalW / 2, mainTextY);
    } else {
      // Inactive: solid fill in the given color with a subtle dark stroke so
      // the text reads clearly on any background. Outline-only rendering (the
      // old behaviour) was too thin to be legible at HUD scale.
      ctx.fillStyle = color;
      ctx.globalAlpha = 0.85;
      ctx.fillText(text, logicalW / 2, mainTextY);
      ctx.globalAlpha = 1;
      ctx.strokeStyle = 'rgba(0,0,0,0.55)';
      ctx.lineWidth = 2;
      ctx.strokeText(text, logicalW / 2, mainTextY);
    }

    if (subtitle) {
      ctx.font = `700 ${subFontSize}px "Orbitron", sans-serif`;
      ctx.fillStyle = 'rgba(148, 163, 184, 0.85)'; // Slate-400 grayish
      ctx.fillText(subtitle, logicalW / 2, centerY + fontSize / 2 + lineGap / 2);
    }

    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    texture.userData.aspect = logicalW / logicalH;
    buttonTextureCache.set(cacheKey, texture);

    const aspect = logicalW / logicalH;
    const height = 0.45;
    const width = height * aspect;
    const geometry = new THREE.PlaneGeometry(width, height);
    const material = new THREE.MeshBasicMaterial({
      map: texture,
      transparent: true,
      side: THREE.DoubleSide,
      depthWrite: false,
      depthTest: true,
      color: 0xffffff,
    });
    material.userData.baseColor = new THREE.Color(0xffffff);

    const mesh = new THREE.Mesh(geometry, material);
    mesh.layers.set(1);
    mesh.renderOrder = 22;
    mesh.userData.isButton = true;
    mesh.userData.actionId = actionId;
    return mesh;
  };

  const create3DLabel = (label: string, color: string = '#f59e0b') => {
    const cacheKey = `label:${label}-${color}-tron-v2`;
    const text = label.toUpperCase();

    // First pass to measure text for dynamic sizing
    const measureCanvas = document.createElement('canvas');
    const measureCtx = measureCanvas.getContext('2d');
    if (!measureCtx) return null;
    const fontSize = 48;
    measureCtx.font = `800 ${fontSize}px "Orbitron", sans-serif`;
    const textMetrics = measureCtx.measureText(text);
    const textWidth = textMetrics.width;

    // Calculate dimensions based on text
    const paddingX = 80;
    const paddingY = 40;
    const logicalW = Math.max(240, textWidth + paddingX * 2);
    const logicalH = 120;
    const pixelRatio = 2;

    const labelW = logicalW / 180; // Scale to world units
    const labelH = logicalH / 180;

    if (buttonTextureCache.has(cacheKey)) {
      const cachedTexture = buttonTextureCache.get(cacheKey)!;
      const geometry = new THREE.PlaneGeometry(labelW, labelH);
      const material = new THREE.MeshBasicMaterial({
        map: cachedTexture,
        transparent: true,
        side: THREE.DoubleSide,
        depthWrite: false,
        depthTest: true,
      });
      const mesh = new THREE.Mesh(geometry, material);
      mesh.layers.set(1);
      mesh.renderOrder = 18;
      mesh.userData.isButton = false;
      return mesh;
    }

    const canvas = document.createElement('canvas');
    canvas.width = logicalW * pixelRatio;
    canvas.height = logicalH * pixelRatio;
    const ctx = canvas.getContext('2d');
    if (!ctx) return null;
    ctx.scale(pixelRatio, pixelRatio);

    // Tron Style: Angular background
    ctx.clearRect(0, 0, logicalW, logicalH);

    const notch = 20;
    const drawAngularPath = (offset: number) => {
      ctx.beginPath();
      ctx.moveTo(offset + notch, offset);
      ctx.lineTo(logicalW - offset - notch, offset);
      ctx.lineTo(logicalW - offset, offset + notch);
      ctx.lineTo(logicalW - offset, logicalH - offset - notch);
      ctx.lineTo(logicalW - offset - notch, logicalH - offset);
      ctx.lineTo(offset + notch, logicalH - offset);
      ctx.lineTo(offset, logicalH - offset - notch);
      ctx.lineTo(offset, offset + notch);
      ctx.closePath();
    };

    // Fill semi-transparent dark background
    ctx.fillStyle = 'rgba(4, 10, 20, 0.9)';
    drawAngularPath(4);
    ctx.fill();

    // Subtle Grid pattern
    ctx.save();
    drawAngularPath(4);
    ctx.clip();
    ctx.strokeStyle = color;
    ctx.globalAlpha = 0.03;
    ctx.lineWidth = 1;
    for (let i = 0; i < logicalW; i += 20) {
      ctx.beginPath();
      ctx.moveTo(i, 0);
      ctx.lineTo(i, logicalH);
      ctx.stroke();
    }
    for (let i = 0; i < logicalH; i += 20) {
      ctx.beginPath();
      ctx.moveTo(0, i);
      ctx.lineTo(logicalW, i);
      ctx.stroke();
    }
    ctx.restore();
    ctx.globalAlpha = 1;

    // Outer Glow Border (Reduced)
    ctx.strokeStyle = color;
    ctx.lineWidth = 2.5;
    ctx.shadowColor = color;
    ctx.shadowBlur = 6;
    drawAngularPath(4);
    ctx.stroke();
    ctx.shadowBlur = 0;

    // Inner Accent Border
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.2)';
    ctx.lineWidth = 1;
    drawAngularPath(8);
    ctx.stroke();

    // Corner Accents (L-shapes) - Less bright
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.7)';
    ctx.lineWidth = 3;
    const cornerSize = 12;
    // Top Left
    ctx.beginPath();
    ctx.moveTo(4, 4 + notch + cornerSize);
    ctx.lineTo(4, 4 + notch);
    ctx.lineTo(4 + notch, 4);
    ctx.lineTo(4 + notch + cornerSize, 4);
    ctx.stroke();
    // Bottom Right
    ctx.beginPath();
    ctx.moveTo(logicalW - 4, logicalH - 4 - notch - cornerSize);
    ctx.lineTo(logicalW - 4, logicalH - 4 - notch);
    ctx.lineTo(logicalW - 4 - notch, logicalH - 4);
    ctx.lineTo(logicalW - 4 - notch - cornerSize, logicalH - 4);
    ctx.stroke();

    // Text with Glow (Reduced)
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.font = `800 ${fontSize}px "Orbitron", sans-serif`;
    ctx.fillStyle = '#fff';
    ctx.shadowColor = color;
    ctx.shadowBlur = 4;
    ctx.fillText(text, logicalW / 2, logicalH / 2);
    ctx.shadowBlur = 0;

    // Add decorative "HUD" elements
    ctx.font = '700 9px monospace';
    ctx.fillStyle = color;
    ctx.globalAlpha = 0.4;
    ctx.fillText('STATUS: ACTIVE', logicalW / 2, 22);
    ctx.fillText('SYS_UID: 0x' + Math.random().toString(16).slice(2, 8).toUpperCase(), logicalW / 2, logicalH - 18);
    ctx.globalAlpha = 1;

    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    buttonTextureCache.set(cacheKey, texture);

    const geometry = new THREE.PlaneGeometry(labelW, labelH);
    const material = new THREE.MeshBasicMaterial({
      map: texture,
      transparent: true,
      side: THREE.DoubleSide,
      depthWrite: false,
      depthTest: true,
    });

    const mesh = new THREE.Mesh(geometry, material);
    mesh.layers.set(1);
    mesh.renderOrder = 18;
    mesh.userData.isButton = false;
    return mesh;
  };

  const createBillboardLabel = (
    title: string,
    subtitle?: string | null,
    color: string = '#f59e0b',
  ) => {
    const canvas = document.createElement('canvas');
    const pixelRatio = 2;
    const ctx = canvas.getContext('2d');
    if (!ctx) return null;

    const padX = 40;
    const padY = 22;
    const gap = subtitle ? 10 : 0;
    const titleFont = '800 34px "Orbitron", sans-serif';
    const subtitleFont = '600 22px "Orbitron", sans-serif';

    ctx.font = titleFont;
    const titleWidth = ctx.measureText(title.toUpperCase()).width;
    let subtitleWidth = 0;
    if (subtitle) {
      ctx.font = subtitleFont;
      subtitleWidth = ctx.measureText(subtitle.toUpperCase()).width;
    }
    const textWidth = Math.max(titleWidth, subtitleWidth);
    const logicalW = Math.max(320, Math.ceil(textWidth + padX * 2));
    const logicalH = subtitle ? padY * 2 + 40 + 26 + gap : padY * 2 + 40;

    canvas.width = Math.round(logicalW * pixelRatio);
    canvas.height = Math.round(logicalH * pixelRatio);
    ctx.scale(pixelRatio, pixelRatio);
    ctx.clearRect(0, 0, logicalW, logicalH);

    const radius = 20;
    ctx.fillStyle = 'rgba(8, 12, 18, 0.7)';
    ctx.strokeStyle = color;
    ctx.lineWidth = 2;
    ctx.shadowBlur = 0;
    ctx.beginPath();
    ctx.moveTo(radius, 0);
    ctx.lineTo(logicalW - radius, 0);
    ctx.quadraticCurveTo(logicalW, 0, logicalW, radius);
    ctx.lineTo(logicalW, logicalH - radius);
    ctx.quadraticCurveTo(logicalW, logicalH, logicalW - radius, logicalH);
    ctx.lineTo(radius, logicalH);
    ctx.quadraticCurveTo(0, logicalH, 0, logicalH - radius);
    ctx.lineTo(0, radius);
    ctx.quadraticCurveTo(0, 0, radius, 0);
    ctx.closePath();
    ctx.fill();
    ctx.stroke();
    ctx.globalAlpha = 0.5;
    ctx.strokeStyle = 'rgba(255,255,255,0.65)';
    ctx.stroke();
    ctx.globalAlpha = 1;

    ctx.fillStyle = '#e2f5ff';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.font = titleFont;
    const titleY = subtitle ? padY + 20 : logicalH / 2;
    ctx.fillText(title.toUpperCase(), logicalW / 2, titleY);

    if (subtitle) {
      ctx.font = subtitleFont;
      ctx.fillStyle = 'rgba(226, 245, 255, 0.75)';
      ctx.fillText(subtitle.toUpperCase(), logicalW / 2, padY + 20 + 40 + gap);
    }

    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    texture.generateMipmaps = false;
    texture.minFilter = THREE.LinearFilter;
    texture.magFilter = THREE.LinearFilter;

    const material = new THREE.SpriteMaterial({
      map: texture,
      transparent: true,
      depthTest: false,
      depthWrite: false,
    });
    const sprite = new THREE.Sprite(material);
    const heightUnits = 0.3;
    const widthUnits = heightUnits * (logicalW / logicalH);
    sprite.scale.set(widthUnits, heightUnits, 1);
    sprite.renderOrder = 80;
    (sprite as any).userData = {
      width: widthUnits,
      height: heightUnits,
    };
    return sprite;
  };

  const createDeckInfoBillboard = (
    name: string,
    description: string | null | undefined,
    colors: string[],
    subtitle?: string | null,
  ) => {
    const canvas = document.createElement('canvas');
    const pixelRatio = 2;
    const ctx = canvas.getContext('2d');
    if (!ctx) return null;

    const padX = 50;
    const padY = 36;
    const titleFont = '800 48px "Orbitron", sans-serif';
    const descFont = '500 32px "Orbitron", sans-serif';
    const maxDescWidth = 700;

    // Measure title
    ctx.font = titleFont;
    const dotSpace = colors.length > 0 ? colors.length * 36 + 16 : 0;
    const titleWidth = ctx.measureText(name.toUpperCase()).width + dotSpace;

    // Word-wrap description
    const descLines: string[] = [];
    let descTextHeight = 0;
    if (description) {
      ctx.font = descFont;
      const words = description.split(' ');
      let line = '';
      for (const word of words) {
        const test = line ? `${line} ${word}` : word;
        if (ctx.measureText(test).width > maxDescWidth && line) {
          descLines.push(line);
          line = word;
        } else {
          line = test;
        }
      }
      if (line) descLines.push(line);
      descTextHeight = descLines.length * 42 + 16;
    }

    const subtitleHeight = subtitle ? 42 : 0;

    const logicalW = Math.max(480, Math.ceil(Math.max(titleWidth, maxDescWidth) + padX * 2));
    const logicalH = padY * 2 + 56 + subtitleHeight + descTextHeight;

    canvas.width = Math.round(logicalW * pixelRatio);
    canvas.height = Math.round(logicalH * pixelRatio);
    ctx.scale(pixelRatio, pixelRatio);
    ctx.clearRect(0, 0, logicalW, logicalH);

    // Rounded rect background (inset by margin so border isn't clipped)
    const margin = 6;
    const radius = 16;
    const rx = margin;
    const ry = margin;
    const rw = logicalW - margin * 2;
    const rh = logicalH - margin * 2;
    ctx.fillStyle = 'rgba(2, 4, 8, 0.95)';
    ctx.strokeStyle = colors[0] ?? '#22d3ee';
    ctx.lineWidth = 3;
    ctx.beginPath();
    ctx.moveTo(rx + radius, ry);
    ctx.lineTo(rx + rw - radius, ry);
    ctx.quadraticCurveTo(rx + rw, ry, rx + rw, ry + radius);
    ctx.lineTo(rx + rw, ry + rh - radius);
    ctx.quadraticCurveTo(rx + rw, ry + rh, rx + rw - radius, ry + rh);
    ctx.lineTo(rx + radius, ry + rh);
    ctx.quadraticCurveTo(rx, ry + rh, rx, ry + rh - radius);
    ctx.lineTo(rx, ry + radius);
    ctx.quadraticCurveTo(rx, ry, rx + radius, ry);
    ctx.closePath();
    ctx.fill();
    ctx.stroke();

    // Color dots
    let dotX = padX;
    const dotY = padY + 26;
    for (const c of colors) {
      ctx.beginPath();
      ctx.arc(dotX + 12, dotY, 12, 0, Math.PI * 2);
      ctx.fillStyle = c;
      ctx.fill();
      ctx.strokeStyle = 'rgba(255,255,255,0.3)';
      ctx.lineWidth = 2;
      ctx.stroke();
      dotX += 36;
    }

    // Title
    ctx.font = titleFont;
    ctx.fillStyle = '#e2f5ff';
    ctx.textAlign = 'left';
    ctx.textBaseline = 'middle';
    ctx.fillText(name.toUpperCase(), dotX + 6, dotY);

    let textY = padY + 56 + 20;

    if (subtitle) {
      ctx.font = '700 28px "Orbitron", sans-serif';
      ctx.fillStyle = 'rgba(226, 245, 255, 0.92)';
      ctx.fillText(subtitle.toUpperCase(), padX, textY);
      textY += 42;
    }

    // Description lines
    if (descLines.length > 0) {
      ctx.font = descFont;
      ctx.fillStyle = 'rgba(148, 163, 184, 0.9)';
      for (let i = 0; i < descLines.length; i++) {
        ctx.fillText(descLines[i], padX, textY + i * 42);
      }
    }

    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    texture.generateMipmaps = false;
    texture.minFilter = THREE.LinearFilter;
    texture.magFilter = THREE.LinearFilter;

    const material = new THREE.SpriteMaterial({
      map: texture,
      transparent: true,
      depthTest: false,
      depthWrite: false,
    });
    const sprite = new THREE.Sprite(material);
    const heightUnits = 1.8;
    const widthUnits = heightUnits * (logicalW / logicalH);
    sprite.scale.set(widthUnits, heightUnits, 1);
    sprite.renderOrder = 80;
    (sprite as any).userData = {
      width: widthUnits,
      height: heightUnits,
    };
    return sprite;
  };

  const createFilterTextLabel = (
    label: string,
    color: string,
    filled: boolean,
  ) => {
    const canvas = document.createElement('canvas');
    const pixelRatio = 2;
    const ctx = canvas.getContext('2d');
    if (!ctx) return null;

    const text = label.toUpperCase();
    const fontSize = 34;
    const font = `700 ${fontSize}px "Orbitron", sans-serif`;
    const padX = 50;
    const padY = 26;
    ctx.font = font;
    const textWidth = ctx.measureText(text).width;
    const logicalW = Math.ceil(textWidth + padX * 2);
    const logicalH = Math.ceil(fontSize + padY * 2 + 14);

    canvas.width = logicalW * pixelRatio;
    canvas.height = logicalH * pixelRatio;
    ctx.scale(pixelRatio, pixelRatio);
    ctx.clearRect(0, 0, logicalW, logicalH);

    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.font = font;
    if (filled) {
      ctx.fillStyle = color;
      ctx.shadowColor = color;
      ctx.shadowBlur = 4;
      ctx.fillText(text, logicalW / 2, logicalH / 2 + 1);
      ctx.shadowBlur = 0;
      ctx.strokeStyle = 'rgba(255,255,255,0.6)';
      ctx.lineWidth = 1.5;
      ctx.strokeText(text, logicalW / 2, logicalH / 2 + 1);
    } else {
      ctx.strokeStyle = color;
      ctx.lineWidth = 2;
      ctx.globalAlpha = 0.75;
      ctx.strokeText(text, logicalW / 2, logicalH / 2 + 1);
      ctx.globalAlpha = 1;
    }

    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    texture.generateMipmaps = false;
    texture.minFilter = THREE.LinearFilter;
    texture.magFilter = THREE.LinearFilter;

    const material = new THREE.SpriteMaterial({
      map: texture,
      transparent: true,
      depthTest: false,
      depthWrite: false,
    });
    const sprite = new THREE.Sprite(material);
    const heightUnits = 0.36;
    const widthUnits = heightUnits * (logicalW / logicalH);
    sprite.scale.set(widthUnits, heightUnits, 1);
    sprite.renderOrder = 60;
    return sprite;
  };

  const createBillboardButton = (label: string, color: string = '#f59e0b') => {
    const canvas = document.createElement('canvas');
    const pixelRatio = 2;
    const ctx = canvas.getContext('2d');
    if (!ctx) return null;

    const logicalW = 220;
    const logicalH = 240;
    canvas.width = logicalW * pixelRatio;
    canvas.height = logicalH * pixelRatio;
    ctx.scale(pixelRatio, pixelRatio);

    const radius = 28;
    ctx.clearRect(0, 0, logicalW, logicalH);
    ctx.fillStyle = 'rgba(8, 12, 18, 0.8)';
    ctx.strokeStyle = color;
    ctx.lineWidth = 5;
    ctx.shadowColor = color;
    ctx.shadowBlur = 10;
    ctx.beginPath();
    ctx.moveTo(radius, 0);
    ctx.lineTo(logicalW - radius, 0);
    ctx.quadraticCurveTo(logicalW, 0, logicalW, radius);
    ctx.lineTo(logicalW, logicalH - radius);
    ctx.quadraticCurveTo(logicalW, logicalH, logicalW - radius, logicalH);
    ctx.lineTo(radius, logicalH);
    ctx.quadraticCurveTo(0, logicalH, 0, logicalH - radius);
    ctx.lineTo(0, radius);
    ctx.quadraticCurveTo(0, 0, radius, 0);
    ctx.closePath();
    ctx.fill();
    ctx.stroke();
    ctx.shadowBlur = 0;

    ctx.fillStyle = '#e2f5ff';
    ctx.font = '800 104px "Orbitron", sans-serif';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(label, logicalW / 2, logicalH / 2 + 2);

    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    texture.generateMipmaps = false;
    texture.minFilter = THREE.LinearFilter;
    texture.magFilter = THREE.LinearFilter;

    const material = new THREE.SpriteMaterial({
      map: texture,
      transparent: true,
      depthTest: false,
      depthWrite: false,
    });
    const sprite = new THREE.Sprite(material);
    sprite.scale.set(0.14, 0.104, 1);
    sprite.renderOrder = 81;
    return sprite;
  };

  const ptBillboardCache = new Map<string, THREE.CanvasTexture>();
  const statHexBadgeCache = new Map<string, THREE.CanvasTexture>();

  /**
   * Small hex-shaped badge showing a single combat stat (power or toughness).
   * Used as a pair floating outside the card mesh so stats can be read even
   * when the card art is obscured. Color-coded vs the base stat: green when
   * buffed, red when debuffed, slate when unchanged.
   */
  const createStatHexBadge = (
    value: number,
    base: number | null,
    kind: 'power' | 'toughness',
    damage: number = 0,
  ) => {
    const delta = base == null ? 0 : value - base;
    const state = delta > 0 ? 'buff' : delta < 0 ? 'debuff' : 'base';
    // Toughness shows "remaining / total" when damage has been dealt so
    // you can tell at a glance how close a unit is to dying.
    const showRemaining = kind === 'toughness' && damage > 0;
    const remaining = Math.max(0, value - damage);
    const cacheKey = `hex:${kind}:${value}:${state}:${showRemaining ? damage : 0}`;

    let texture = statHexBadgeCache.get(cacheKey);
    if (!texture) {
      const size = 128;
      const canvas = document.createElement('canvas');
      canvas.width = size;
      canvas.height = size;
      const ctx = canvas.getContext('2d');
      if (!ctx) return null;

      const cx = size / 2;
      const cy = size / 2;
      const r = size * 0.46;

      // Color scheme per kind + state
      const palettes = {
        power: {
          base: { fill: '#7f1d1d', stroke: '#fca5a5', glow: '#ef4444', text: '#fef2f2' },
          buff: { fill: '#064e3b', stroke: '#6ee7b7', glow: '#10b981', text: '#ecfdf5' },
          debuff: { fill: '#450a0a', stroke: '#fda4af', glow: '#f43f5e', text: '#fff1f2' },
        },
        toughness: {
          base: { fill: '#0c4a6e', stroke: '#7dd3fc', glow: '#0ea5e9', text: '#f0f9ff' },
          buff: { fill: '#064e3b', stroke: '#6ee7b7', glow: '#10b981', text: '#ecfdf5' },
          debuff: { fill: '#450a0a', stroke: '#fda4af', glow: '#f43f5e', text: '#fff1f2' },
        },
      } as const;
      // Damaged toughness tints red regardless of buff/debuff to flag near-death.
      const pal = showRemaining
        ? palettes.toughness.debuff
        : palettes[kind][state];

      const hex = (fillOnly: boolean) => {
        ctx.beginPath();
        for (let i = 0; i < 6; i++) {
          const angle = (i * Math.PI) / 3 - Math.PI / 2;
          const px = cx + Math.cos(angle) * r;
          const py = cy + Math.sin(angle) * r;
          if (i === 0) ctx.moveTo(px, py);
          else ctx.lineTo(px, py);
        }
        ctx.closePath();
        if (fillOnly) ctx.fill();
        else ctx.stroke();
      };

      // Outer glow
      ctx.shadowColor = pal.glow;
      ctx.shadowBlur = 18;
      ctx.fillStyle = pal.fill;
      hex(true);
      ctx.shadowBlur = 0;

      // Ring
      ctx.strokeStyle = pal.stroke;
      ctx.lineWidth = 4;
      hex(false);

      // Number — show "remaining/total" on damaged toughness, single number otherwise.
      ctx.fillStyle = pal.text;
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      if (showRemaining) {
        ctx.font = '900 42px "Orbitron", sans-serif';
        ctx.fillText(`${remaining}/${value}`, cx, cy + 2);
      } else {
        ctx.font = '900 64px "Orbitron", sans-serif';
        ctx.fillText(String(value), cx, cy + 2);
      }

      texture = new THREE.CanvasTexture(canvas);
      texture.colorSpace = THREE.SRGBColorSpace;
      texture.generateMipmaps = false;
      texture.minFilter = THREE.LinearFilter;
      texture.magFilter = THREE.LinearFilter;
      statHexBadgeCache.set(cacheKey, texture);
    }

    const material = new THREE.SpriteMaterial({
      map: texture,
      transparent: true,
      depthTest: false,
      depthWrite: false,
    });
    const sprite = new THREE.Sprite(material);
    sprite.scale.set(0.26, 0.26, 1);
    sprite.renderOrder = 102;
    return sprite;
  };

  // Bolt glyph (power / attack).
  const drawBoltGlyph = (
    ctx: CanvasRenderingContext2D,
    cx: number,
    cy: number,
    g: number,
    color: string,
  ) => {
    ctx.save();
    ctx.fillStyle = color;
    ctx.beginPath();
    ctx.moveTo(cx + g * 0.1, cy - g * 0.65);
    ctx.lineTo(cx - g * 0.35, cy + g * 0.05);
    ctx.lineTo(cx - g * 0.05, cy + g * 0.05);
    ctx.lineTo(cx - g * 0.2, cy + g * 0.65);
    ctx.lineTo(cx + g * 0.35, cy - g * 0.05);
    ctx.lineTo(cx + g * 0.05, cy - g * 0.05);
    ctx.closePath();
    ctx.fill();
    ctx.restore();
  };

  // Hexagon glyph (toughness / defense).
  const drawHexGlyph = (
    ctx: CanvasRenderingContext2D,
    cx: number,
    cy: number,
    g: number,
    color: string,
  ) => {
    ctx.save();
    ctx.translate(cx, cy);
    ctx.strokeStyle = color;
    ctx.fillStyle = color + '33';
    ctx.lineWidth = Math.max(1.5, g * 0.12);
    ctx.lineJoin = 'round';
    const r = g * 0.52;
    ctx.beginPath();
    for (let i = 0; i < 6; i++) {
      const angle = (i * Math.PI) / 3 - Math.PI / 2;
      const px = Math.cos(angle) * r;
      const py = Math.sin(angle) * r;
      if (i === 0) ctx.moveTo(px, py);
      else ctx.lineTo(px, py);
    }
    ctx.closePath();
    ctx.fill();
    ctx.stroke();
    ctx.restore();
  };

  /**
   * Shared single-pill P/T renderer: bolt glyph + power number on
   * the left half, hexagon glyph + toughness text on the right half, inside
   * a rounded pill. Border is amber if modified, slate if neutral.
   */
  const drawPTPill = (
    ctx: CanvasRenderingContext2D,
    x: number,
    y: number,
    w: number,
    h: number,
    power: number,
    toughText: string,
    powerColor: string,
    toughColor: string,
    borderColor: string,
    unit: number, // reference size for fonts/glyphs, e.g. h
  ) => {
    ctx.save();
    // Background pill
    ctx.fillStyle = 'rgba(5, 10, 18, 0.92)';
    ctx.beginPath();
    ctx.roundRect(x, y, w, h, unit * 0.22);
    ctx.fill();

    // Border glow
    ctx.strokeStyle = borderColor;
    ctx.lineWidth = Math.max(1, unit * 0.025);
    ctx.shadowColor = borderColor;
    ctx.shadowBlur = unit * 0.1;
    ctx.stroke();
    ctx.shadowBlur = 0;

    const cy = y + h / 2;
    const halfW = w / 2;
    const glyphSize = unit * 0.36;

    // Power cell — glyph on left of number.
    const powerGlyphX = x + halfW * 0.3;
    const powerNumX = x + halfW * 0.55;
    drawBoltGlyph(ctx, powerGlyphX, cy, glyphSize, powerColor);
    ctx.fillStyle = powerColor;
    ctx.font = `900 ${unit * 0.52}px "Orbitron", sans-serif`;
    ctx.textAlign = 'left';
    ctx.textBaseline = 'middle';
    ctx.shadowColor = powerColor;
    ctx.shadowBlur = unit * 0.08;
    ctx.fillText(`${power}`, powerNumX, cy + unit * 0.02);
    ctx.shadowBlur = 0;

    // Toughness cell — glyph on left of number.
    const toughGlyphX = x + halfW + halfW * 0.3;
    const toughNumX = x + halfW + halfW * 0.55;
    drawHexGlyph(ctx, toughGlyphX, cy, glyphSize, toughColor);
    ctx.fillStyle = toughColor;
    const tFont = toughText.length > 3 ? unit * 0.36 : unit * 0.52;
    ctx.font = `900 ${tFont}px "Orbitron", sans-serif`;
    ctx.textAlign = 'left';
    ctx.textBaseline = 'middle';
    ctx.shadowColor = toughColor;
    ctx.shadowBlur = unit * 0.08;
    ctx.fillText(toughText, toughNumX, cy + unit * 0.02);
    ctx.shadowBlur = 0;
    ctx.restore();
  };

  const createPTBillboard = (
    power: number,
    toughness: number,
    basePower: number | null,
    baseToughness: number | null,
    cardW: number = 1.0,
    damage: number = 0,
  ) => {
    const pBase = basePower ?? power;
    const tBase = baseToughness ?? toughness;

    const powerBuff = power > pBase;
    const powerDebuff = power < pBase;
    const toughBuff = toughness > tBase;
    const toughDebuff = toughness < tBase || damage > 0;

    const neutralColor = '#94a3b8'; // Slate-400
    const powerColor = powerBuff ? '#4ade80' : powerDebuff ? '#f87171' : neutralColor;
    const toughColor = toughBuff ? '#4ade80' : toughDebuff ? '#f87171' : neutralColor;

    const isModified = powerBuff || powerDebuff || toughBuff || toughDebuff;
    const borderColor = isModified ? '#f59e0b' : '#475569'; // Amber glow if modified, muted slate if neutral

    const remaining = Math.max(0, toughness - damage);
    const toughText = `${damage > 0 ? remaining : toughness}`;
    const cacheKey = `pt3:${power}:${toughText}:${powerColor}:${toughColor}:${borderColor}`;

    let texture = ptBillboardCache.get(cacheKey);
    if (!texture) {
      const canvasW = 240;
      const canvasH = 96;
      const canvas = document.createElement('canvas');
      canvas.width = canvasW;
      canvas.height = canvasH;
      const ctx = canvas.getContext('2d');
      if (!ctx) return null;

      ctx.clearRect(0, 0, canvasW, canvasH);
      drawPTPill(
        ctx,
        4,
        4,
        canvasW - 8,
        canvasH - 8,
        power,
        toughText,
        powerColor,
        toughColor,
        borderColor,
        canvasH - 8,
      );

      texture = new THREE.CanvasTexture(canvas);
      texture.colorSpace = THREE.SRGBColorSpace;
      texture.generateMipmaps = false;
      texture.minFilter = THREE.LinearFilter;
      texture.magFilter = THREE.LinearFilter;
      ptBillboardCache.set(cacheKey, texture);
    }

    const material = new THREE.SpriteMaterial({
      map: texture,
      transparent: true,
      depthTest: false,
      depthWrite: false,
    });
    const sprite = new THREE.Sprite(material);
    const widthUnits = Math.min(0.55, cardW * 0.62);
    const heightUnits = widthUnits * (96 / 240);
    sprite.scale.set(widthUnits, heightUnits, 1);
    sprite.renderOrder = 9999;
    return sprite;
  };

  const createPlayerSeatCard = (
    playerName: string,
    deckName: string | null,
    isReady: boolean,
    options?: {
      color?: string;
      width?: number;
      height?: number;
    },
  ) => {
    const color = options?.color ?? '#f59e0b';
    const pixelRatio = 2;
    const logicalW = 480;
    const logicalH = 200;
    const cacheKey = `seat:${playerName}-${deckName ?? 'none'}-${isReady}-${color}-v2`;

    let texture: THREE.CanvasTexture;
    if (buttonTextureCache.has(cacheKey)) {
      texture = buttonTextureCache.get(cacheKey)!;
    } else {
      const canvas = document.createElement('canvas');
      canvas.width = Math.round(logicalW * pixelRatio);
      canvas.height = Math.round(logicalH * pixelRatio);
      const ctx = canvas.getContext('2d');
      if (!ctx) return null;
      ctx.scale(pixelRatio, pixelRatio);
      ctx.clearRect(0, 0, logicalW, logicalH);

      // Background
      const radius = 12;
      const drawRounded = (inset: number) => {
        const x = inset, y = inset, w = logicalW - inset * 2, h = logicalH - inset * 2;
        ctx.beginPath();
        ctx.moveTo(x + radius, y);
        ctx.lineTo(x + w - radius, y);
        ctx.quadraticCurveTo(x + w, y, x + w, y + radius);
        ctx.lineTo(x + w, y + h - radius);
        ctx.quadraticCurveTo(x + w, y + h, x + w - radius, y + h);
        ctx.lineTo(x + radius, y + h);
        ctx.quadraticCurveTo(x, y + h, x, y + h - radius);
        ctx.lineTo(x, y + radius);
        ctx.quadraticCurveTo(x, y, x + radius, y);
        ctx.closePath();
      };

      ctx.fillStyle = 'rgba(4, 10, 20, 0.88)';
      drawRounded(3);
      ctx.fill();

      // Border with glow
      ctx.strokeStyle = color;
      ctx.lineWidth = 2;
      ctx.shadowColor = color;
      ctx.shadowBlur = 8;
      drawRounded(3);
      ctx.stroke();
      ctx.shadowBlur = 0;

      // Inner accent
      ctx.strokeStyle = 'rgba(255, 255, 255, 0.15)';
      ctx.lineWidth = 1;
      drawRounded(6);
      ctx.stroke();

      // Player name
      ctx.font = '800 36px "Orbitron", sans-serif';
      ctx.fillStyle = '#e2f5ff';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(playerName.toUpperCase(), logicalW / 2, logicalH * 0.35);

      // Deck name or waiting
      ctx.font = '600 24px "Orbitron", sans-serif';
      if (deckName) {
        ctx.fillStyle = 'rgba(148, 163, 184, 0.9)';
        ctx.fillText(deckName, logicalW / 2, logicalH * 0.58);
      } else {
        ctx.fillStyle = 'rgba(100, 116, 139, 0.7)';
        ctx.fillText('Waiting...', logicalW / 2, logicalH * 0.58);
      }

      // Ready indicator
      if (isReady) {
        ctx.font = '700 20px "Orbitron", sans-serif';
        ctx.fillStyle = '#4ade80';
        ctx.shadowColor = '#4ade80';
        ctx.shadowBlur = 6;
        ctx.fillText('READY \u2713', logicalW / 2, logicalH * 0.8);
        ctx.shadowBlur = 0;
      }

      texture = new THREE.CanvasTexture(canvas);
      texture.colorSpace = THREE.SRGBColorSpace;
      texture.generateMipmaps = false;
      texture.minFilter = THREE.LinearFilter;
      texture.magFilter = THREE.LinearFilter;
      buttonTextureCache.set(cacheKey, texture);
    }

    // Billboard sprite — always faces camera
    const material = new THREE.SpriteMaterial({
      map: texture,
      transparent: true,
      depthTest: true,
      depthWrite: false,
    });
    const sprite = new THREE.Sprite(material);
    const heightUnits = options?.height ?? 1.2;
    const widthUnits = heightUnits * (logicalW / logicalH);
    sprite.scale.set(widthUnits, heightUnits, 1);
    sprite.renderOrder = 18;
    sprite.userData.isButton = false;
    return sprite;
  };

  const create3DTextButtonWithIcon = (
    label: string,
    iconText: string,
    actionId: string,
    color: string = '#f59e0b',
    active: boolean = false,
    options?: {
      fontSize?: number;
      pixelRatio?: number;
    },
  ) => {
    const text = label.toUpperCase();
    const pixelRatio = options?.pixelRatio ?? 2;
    const fontSize = options?.fontSize ?? 36;
    const iconFontSize = Math.round(fontSize * 0.75);
    const cacheKey = `txt-btn-icon-svg:${text}-${iconText}-${color}-${active}-${fontSize}`;

    if (buttonTextureCache.has(cacheKey)) {
      const cachedTexture = buttonTextureCache.get(cacheKey)!;
      const aspect = cachedTexture.userData.aspect ?? 2;
      const height = 0.45;
      const width = height * aspect;
      const geometry = new THREE.PlaneGeometry(width, height);
      const material = new THREE.MeshBasicMaterial({
        map: cachedTexture,
        transparent: true,
        side: THREE.DoubleSide,
        depthWrite: false,
        depthTest: true,
        color: 0xffffff,
      });
      material.userData.baseColor = new THREE.Color(0xffffff);
      const mesh = new THREE.Mesh(geometry, material);
      mesh.layers.set(1);
      mesh.renderOrder = 22;
      mesh.userData.isButton = true;
      mesh.userData.actionId = actionId;
      return mesh;
    }

    // Measure text
    const tempCanvas = document.createElement('canvas');
    const tempCtx = tempCanvas.getContext('2d')!;
    tempCtx.font = `800 ${fontSize}px "Orbitron", sans-serif`;
    const mainWidth = tempCtx.measureText(text).width;
    
    // Parse out text vs icon space. If we pass "♡ 30", we'll just ignore the heart and draw the number, and manually draw the SVG
    const numText = iconText.replace(/[^0-9]/g, '').trim(); 
    tempCtx.font = `700 ${iconFontSize}px "Orbitron", sans-serif`;
    const numberWidth = numText ? tempCtx.measureText(` ${numText}`).width : 0;
    
    const iconDrawSize = fontSize * 0.8;
    const gap = 16;
    const iconGap = 8;
    const totalWidth = mainWidth + gap + iconDrawSize + numberWidth;

    const paddingX = 24;
    const paddingY = 15;
    const logicalW = totalWidth + paddingX * 2;
    const logicalH = fontSize + paddingY * 2;

    const canvas = document.createElement('canvas');
    canvas.width = Math.round(logicalW * pixelRatio);
    canvas.height = Math.round(logicalH * pixelRatio);
    const ctx = canvas.getContext('2d');
    if (!ctx) return null;
    ctx.scale(pixelRatio, pixelRatio);

    ctx.clearRect(0, 0, logicalW, logicalH);
    ctx.textBaseline = 'middle';
    const centerY = logicalH / 2;

    // Main label (left-aligned within centered group)
    const groupStartX = (logicalW - totalWidth) / 2;

    ctx.font = `800 ${fontSize}px "Orbitron", sans-serif`;
    ctx.textAlign = 'left';
    if (active) {
      ctx.fillStyle = color;
      ctx.shadowColor = color;
      ctx.shadowBlur = 10;
      ctx.fillText(text, groupStartX, centerY);
      ctx.shadowBlur = 0;
      ctx.strokeStyle = 'rgba(255,255,255,0.8)';
      ctx.lineWidth = 1;
      ctx.strokeText(text, groupStartX, centerY);
    } else {
      ctx.strokeStyle = color;
      ctx.lineWidth = 2;
      ctx.globalAlpha = 0.7;
      ctx.strokeText(text, groupStartX, centerY);
      ctx.globalAlpha = 1;
    }

    const iconX = groupStartX + mainWidth + gap;
    const iconColor = active ? '#4ade80' : 'rgba(74, 222, 128, 0.6)';
    
    // Draw SVG Activity path
    ctx.save();
    ctx.translate(iconX, centerY - iconDrawSize / 2);
    // Scale 24x24 viewBox to iconDrawSize
    const scale = iconDrawSize / 24;
    ctx.scale(scale, scale);
    
    const path = new Path2D("M22 12h-2.48a2 2 0 0 0-1.93 1.46l-2.35 8.36a.25.25 0 0 1-.48 0L9.24 2.18a.25.25 0 0 0-.48 0l-2.35 8.36A2 2 0 0 1 4.49 12H2");
    ctx.strokeStyle = iconColor;
    ctx.lineWidth = 2.5; // Slightly thicker for clarity
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    if (active) {
      ctx.shadowColor = '#4ade80';
      ctx.shadowBlur = 6;
    }
    ctx.stroke(path);
    ctx.restore();

    if (numText) {
      ctx.font = `700 ${iconFontSize}px "Orbitron", sans-serif`;
      ctx.fillStyle = iconColor;
      if (active) {
        ctx.shadowColor = '#4ade80';
        ctx.shadowBlur = 6;
      }
      ctx.fillText(` ${numText}`, iconX + iconDrawSize, centerY);
      ctx.shadowBlur = 0;
    }

    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    texture.userData.aspect = logicalW / logicalH;
    buttonTextureCache.set(cacheKey, texture);

    const aspect = logicalW / logicalH;
    const height = 0.45;
    const width = height * aspect;
    const geometry = new THREE.PlaneGeometry(width, height);
    const material = new THREE.MeshBasicMaterial({
      map: texture,
      transparent: true,
      side: THREE.DoubleSide,
      depthWrite: false,
      depthTest: true,
      color: 0xffffff,
    });
    material.userData.baseColor = new THREE.Color(0xffffff);

    const mesh = new THREE.Mesh(geometry, material);
    mesh.layers.set(1);
    mesh.renderOrder = 22;
    mesh.userData.isButton = true;
    mesh.userData.actionId = actionId;
    return mesh;
  };

  const createMalwareBillboard = () => {
    const canvas = document.createElement('canvas');
    canvas.width = 256;
    canvas.height = 96;
    const ctx = canvas.getContext('2d');
    if (!ctx) return null;
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    // Pulsing magenta/violet badge — "infected" look
    const grad = ctx.createLinearGradient(0, 0, 0, canvas.height);
    grad.addColorStop(0, 'rgba(168, 85, 247, 0.95)');
    grad.addColorStop(1, 'rgba(76, 29, 149, 0.95)');
    ctx.fillStyle = grad;
    const r = 14;
    ctx.beginPath();
    ctx.moveTo(r, 0);
    ctx.arcTo(canvas.width, 0, canvas.width, canvas.height, r);
    ctx.arcTo(canvas.width, canvas.height, 0, canvas.height, r);
    ctx.arcTo(0, canvas.height, 0, 0, r);
    ctx.arcTo(0, 0, canvas.width, 0, r);
    ctx.closePath();
    ctx.fill();

    ctx.strokeStyle = 'rgba(216, 180, 254, 0.9)';
    ctx.lineWidth = 4;
    ctx.stroke();

    ctx.shadowColor = 'rgba(168, 85, 247, 0.8)';
    ctx.shadowBlur = 18;
    ctx.fillStyle = '#ffffff';
    ctx.font = 'bold 44px monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText('MALWARE', canvas.width / 2, canvas.height / 2);
    ctx.shadowBlur = 0;

    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    const material = new THREE.SpriteMaterial({
      map: texture,
      transparent: true,
      depthTest: false,
      depthWrite: false,
    });
    const sprite = new THREE.Sprite(material);
    sprite.renderOrder = 50;
    sprite.scale.set(0.5, 0.19, 1);
    return sprite;
  };

  return {
    createCardBackTexture,
    createPileOutline,
    createCardTexture,
    createStatSprite,
    createStatBarBillboard,
    create3DButton,
    create3DTextButton,
    create3DLabel,
    createBillboardLabel,
    createDeckInfoBillboard,
    createFilterTextLabel,
    createBillboardButton,
    createPTBillboard,
    createStatHexBadge,
    createPlayerSeatCard,
    create3DTextButtonWithIcon,
    createMalwareBillboard,
  };
};
