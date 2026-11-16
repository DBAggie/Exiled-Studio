// Which of the equipment panel's sprites (`sprites.tsx`) an item takes.

/** What a sprite shows. */
export type SpriteKind =
  | 'one-hand'
  | 'two-hand'
  | 'bow'
  | 'wand'
  | 'staff'
  | 'shield'
  | 'quiver'
  | 'helmet'
  | 'body'
  | 'gloves'
  | 'boots'
  | 'belt'
  | 'ring'
  | 'amulet'
  | 'flask'
  | 'jewel'

/** A sprite for an item of `itemClass` (the game data's item class), else
 * for what fits the slot. */
export function spriteFor(
  itemClass: string | null | undefined,
  slot: string,
): SpriteKind {
  switch (itemClass) {
    case 'Bow':
      return 'bow'
    case 'Wand':
      return 'wand'
    case 'Staff':
    case 'Warstaff':
      return 'staff'
    case 'Two Hand Axe':
    case 'Two Hand Mace':
    case 'Two Hand Sword':
      return 'two-hand'
    case 'Shield':
      return 'shield'
    case 'Quiver':
      return 'quiver'
    case 'Jewel':
    case 'AbyssJewel':
      return 'jewel'
    default:
      break
  }
  if (itemClass?.endsWith('Flask') === true || itemClass === 'Tincture') {
    return 'flask'
  }
  if (slot.startsWith('Weapon')) return 'one-hand'
  if (slot.startsWith('Helmet')) return 'helmet'
  if (slot.startsWith('Body')) return 'body'
  if (slot.startsWith('Gloves')) return 'gloves'
  if (slot.startsWith('Boots')) return 'boots'
  if (slot.startsWith('Belt')) return 'belt'
  if (slot.startsWith('Ring')) return 'ring'
  if (slot.startsWith('Amulet')) return 'amulet'
  if (slot.startsWith('Flask')) return 'flask'
  return 'jewel'
}
