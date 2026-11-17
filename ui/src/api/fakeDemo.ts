// Sample numbers for the in-memory API (`FakeApi`): a full set of fields
// that move with the stage, the main gem's level, the character's level,
// and the enemy's level, so the dashboards' charts have something to draw
// when the UI runs in a browser without the shell. Sample data for layout
// only: nothing here is a calculation.
import type { CalcResultDocumentV1 } from './types'

export interface DemoInput {
  /** Life from the sample items (the fake's own small calculation). */
  life: number
  /** The stage, from 0. */
  stage: number
  /** The main gem's level. */
  gemLevel: number
  /** The character's level. */
  level: number
  /** The enemy's level (a config input). */
  enemyLevel: number
}

const round = (n: number, places = 0) => {
  const f = 10 ** places
  return Math.round(n * f) / f
}

/** Every sample field's value for `input`, in the engine's field order. */
export function demoValues(input: DemoInput): [string, number][] {
  const s = 1 + input.stage * 1.6
  const g = input.gemLevel
  const average = (140 + 96 * g + 0.9 * g * g) * s
  const speed = 1.35 + 0.012 * g + 0.2 * input.stage
  const critChance = Math.min(100, 6 + 14 * input.stage + g * 0.4)
  const critMultiplier = 1.5 + 0.45 * input.stage
  // Harder to hit a higher enemy: a few points from level 70 up.
  const hitChance = Math.max(
    60,
    Math.min(100, 100 - (input.enemyLevel - 70) / 4),
  )
  const dps =
    average *
    speed *
    (hitChance / 100) *
    (1 + (critChance / 100) * (critMultiplier - 1))
  const life = input.life + 38 * input.level * (0.6 + 0.4 * s)
  const mana = 180 + 9 * input.level
  const capped = (base: number) => Math.min(75, base + 30 * input.stage)
  return [
    ['life', round(life)],
    ['strength', 20 + 14 * input.stage + Math.floor(input.level / 6)],
    ['dexterity', 5 + 22 * input.stage + Math.floor(input.level / 9)],
    ['intelligence', 20 + 60 * input.stage + Math.floor(input.level / 3)],
    ['mana', round(mana)],
    ['energy-shield', round(140 * s + 6 * input.level)],
    ['armour', round(900 * s)],
    ['evasion', round(650 * s)],
    ['resistance.fire', capped(46)],
    ['resistance.cold', capped(52)],
    ['resistance.lightning', capped(38)],
    ['resistance.chaos', -42 + 28 * input.stage],
    ['block-chance', 12 * input.stage],
    ['life-unreserved', round(life * 0.9)],
    ['mana-unreserved', round(mana * (0.55 - 0.2 * input.stage))],
    ['average-damage', round(average, 1)],
    ['speed', round(speed, 2)],
    ['hit-speed', round(speed, 2)],
    ['hit-chance', round(hitChance, 1)],
    ['crit-chance', round(critChance, 1)],
    ['crit-multiplier', round(critMultiplier, 2)],
    ['hit-dps', round(dps, 1)],
    ['poe1.spell-suppression-chance', 18 * input.stage],
    ['poe1.spell-block-chance', 6 * input.stage],
  ]
}

/** `doc` with the sample fields in place of its small set (the first
 * field, life, keeps its breakdown node). */
export function demoDocument(
  doc: CalcResultDocumentV1,
  input: DemoInput,
): CalcResultDocumentV1 {
  const fields = demoValues(input).map(([field, value]) => ({
    field,
    state: field === 'resistance.chaos' ? 'partial' : 'complete',
    node: null,
    value,
    ...(field === 'resistance.chaos'
      ? { unsupported: ['unchecked-tree-version'] }
      : {}),
  }))
  return {
    ...doc,
    result: { ...doc.result, fields },
  } as unknown as CalcResultDocumentV1
}
