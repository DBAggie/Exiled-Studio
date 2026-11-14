// Simple sprites for the equipment panel, drawn for this project (no game
// art, ADR-000): one per kind of item, in the current colour, with a
// flask's liquid in its own colour.

import type { SpriteKind } from './spriteKind'

/** A flask's liquid colour from its item class. */
function liquid(itemClass: string | null | undefined): string {
  switch (itemClass) {
    case 'LifeFlask':
      return 'var(--c-flask-life)'
    case 'ManaFlask':
      return 'var(--c-flask-mana)'
    case 'HybridFlask':
      return 'var(--c-flask-hybrid)'
    default:
      return 'var(--c-flask-utility)'
  }
}

const PATHS: Record<Exclude<SpriteKind, 'flask'>, string> = {
  'one-hand': 'M30 6 L36 12 L18 46 L14 44 Z M12 40 L22 48 M14 46 L8 56',
  'two-hand':
    'M44 4 L52 10 L22 46 L16 42 Z M10 36 L28 52 M18 46 L6 60 M38 10 L46 18',
  bow: 'M18 4 C46 16 46 48 18 60 M18 4 L18 60 M18 32 L50 32 M44 26 L52 32 L44 38',
  wand: 'M42 8 L48 14 L20 54 L14 50 Z M44 4 L44 0 M50 10 L54 10 M48 6 L52 2',
  staff:
    'M36 4 C46 4 48 14 40 18 L22 58 L18 56 L34 16 C28 12 30 4 36 4 Z M34 10 L38 10',
  shield: 'M32 4 L54 12 C54 34 46 50 32 60 C18 50 10 34 10 12 Z M32 4 L32 60',
  quiver:
    'M20 18 L44 14 L46 58 L22 60 Z M24 18 L20 4 M30 17 L28 2 M36 16 L38 2 M42 15 L46 4',
  helmet:
    'M12 40 C12 18 22 8 32 8 C42 8 52 18 52 40 L46 52 L40 40 L24 40 L18 52 Z M24 30 L40 30',
  body: 'M18 6 L28 10 L36 10 L46 6 L56 18 L48 26 L46 58 L18 58 L16 26 L8 18 Z M28 10 L32 20 L36 10',
  gloves:
    'M18 58 L18 30 L14 18 L18 16 L22 26 L22 8 L27 8 L28 24 L30 6 L35 6 L35 24 L38 8 L43 8 L42 28 L48 22 L52 26 L44 44 L42 58 Z',
  boots: 'M18 6 L36 6 L36 38 L54 46 L56 56 L14 56 L16 40 Z M18 18 L36 18',
  belt: 'M4 22 L60 22 L60 42 L4 42 Z M24 18 L40 18 L40 46 L24 46 Z M30 26 L34 26 L34 38 L30 38 Z',
  ring: 'M32 20 C44 20 50 30 50 38 C50 50 40 58 32 58 C24 58 14 50 14 38 C14 30 20 20 32 20 Z M32 28 C38 28 42 34 42 38 C42 44 38 50 32 50 C26 50 22 44 22 38 C22 34 26 28 32 28 Z M26 20 L32 8 L38 20',
  amulet:
    'M14 4 C14 24 24 32 32 34 C40 32 50 24 50 4 M32 34 L22 46 L32 60 L42 46 Z',
  jewel:
    'M32 6 L54 24 L44 56 L20 56 L10 24 Z M10 24 L54 24 M32 6 L24 24 L32 56 L40 24 Z',
}

/** A sprite: decorative, the slot's label names the item. */
export function Sprite({
  kind,
  itemClass,
}: {
  kind: SpriteKind
  itemClass?: string | null
}) {
  return (
    <svg
      className={`sprite sprite-${kind}`}
      viewBox="0 0 64 64"
      aria-hidden="true"
      focusable="false"
    >
      {kind === 'flask' ? (
        <>
          <path
            d="M22 22 L22 32 L14 46 L14 56 L50 56 L50 46 L42 32 L42 22 Z"
            fill={liquid(itemClass)}
            stroke="none"
            opacity="0.85"
          />
          <path d="M24 4 L40 4 L40 12 L24 12 Z M26 12 L26 30 L16 46 L16 60 L48 60 L48 46 L38 30 L38 12" />
        </>
      ) : (
        <path d={PATHS[kind]} />
      )}
    </svg>
  )
}
