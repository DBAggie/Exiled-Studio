// Generated from schemas/calc-result.v1.schema.json by pnpm generate:schema; do not edit.

/**
 * Why a value cannot be settled.
 */
export type IndeterminateReason =
  | {
      'conflicting-override': {
        /**
         * The quantity.
         */
        quantity: string
      }
    }
  | {
      'unsettled-value': {
        /**
         * Where it comes from.
         */
        source:
          | 'character'
          | {
              item: {
                /**
                 * The import tolerance the line needed, if any.
                 */
                adapted?: Adapted | null
                /**
                 * The item's ID.
                 */
                item: number
                /**
                 * The line's index among the item's lines.
                 */
                line: number
              }
            }
          | {
              'item-base': {
                /**
                 * The item's ID.
                 */
                item: number
              }
            }
          | {
              passive: {
                /**
                 * The node's ID.
                 */
                node: number
              }
            }
          | {
              gem: {
                /**
                 * The gem's index in the group.
                 */
                gem: number
                /**
                 * The socket group's index.
                 */
                group: number
              }
            }
          | {
              config: {
                /**
                 * The input's name.
                 */
                input: string
              }
            }
          | 'enemy'
          | {
              rule: string
            }
        /**
         * The stat ID.
         */
        stat: string
      }
    }
  | {
      'missing-data': {
        /**
         * The key the build gives.
         */
        key: string
        /**
         * The table.
         */
        table: string
      }
    }
  | {
      'tree-version': {
        /**
         * The build's.
         */
        build: string
        /**
         * The data's.
         */
        data: string
      }
    }
/**
 * An import tolerance a line needed.
 */
export type Adapted = 'case-folded' | 'split-section'
/**
 * Why a field is not complete, or not computed.
 */
export type UnsupportedReason =
  | {
      'unmapped-stat': {
        /**
         * Where it comes from.
         */
        source:
          | 'character'
          | {
              item: {
                /**
                 * The import tolerance the line needed, if any.
                 */
                adapted?: Adapted | null
                /**
                 * The item's ID.
                 */
                item: number
                /**
                 * The line's index among the item's lines.
                 */
                line: number
              }
            }
          | {
              'item-base': {
                /**
                 * The item's ID.
                 */
                item: number
              }
            }
          | {
              passive: {
                /**
                 * The node's ID.
                 */
                node: number
              }
            }
          | {
              gem: {
                /**
                 * The gem's index in the group.
                 */
                gem: number
                /**
                 * The socket group's index.
                 */
                group: number
              }
            }
          | {
              config: {
                /**
                 * The input's name.
                 */
                input: string
              }
            }
          | 'enemy'
          | {
              rule: string
            }
        /**
         * The stat ID.
         */
        stat: string
      }
    }
  | {
      'unresolved-line': {
        /**
         * The resolver's reason.
         */
        reason: string
        /**
         * The line.
         */
        source:
          | 'character'
          | {
              item: {
                /**
                 * The import tolerance the line needed, if any.
                 */
                adapted?: Adapted | null
                /**
                 * The item's ID.
                 */
                item: number
                /**
                 * The line's index among the item's lines.
                 */
                line: number
              }
            }
          | {
              'item-base': {
                /**
                 * The item's ID.
                 */
                item: number
              }
            }
          | {
              passive: {
                /**
                 * The node's ID.
                 */
                node: number
              }
            }
          | {
              gem: {
                /**
                 * The gem's index in the group.
                 */
                gem: number
                /**
                 * The socket group's index.
                 */
                group: number
              }
            }
          | {
              config: {
                /**
                 * The input's name.
                 */
                input: string
              }
            }
          | 'enemy'
          | {
              rule: string
            }
      }
    }
  | {
      'excluded-line': {
        /**
         * The exclusion.
         */
        exclusion: string
        /**
         * The line.
         */
        source:
          | 'character'
          | {
              item: {
                /**
                 * The import tolerance the line needed, if any.
                 */
                adapted?: Adapted | null
                /**
                 * The item's ID.
                 */
                item: number
                /**
                 * The line's index among the item's lines.
                 */
                line: number
              }
            }
          | {
              'item-base': {
                /**
                 * The item's ID.
                 */
                item: number
              }
            }
          | {
              passive: {
                /**
                 * The node's ID.
                 */
                node: number
              }
            }
          | {
              gem: {
                /**
                 * The gem's index in the group.
                 */
                gem: number
                /**
                 * The socket group's index.
                 */
                group: number
              }
            }
          | {
              config: {
                /**
                 * The input's name.
                 */
                input: string
              }
            }
          | 'enemy'
          | {
              rule: string
            }
      }
    }
  | {
      'pending-line': {
        /**
         * The line.
         */
        source:
          | 'character'
          | {
              item: {
                /**
                 * The import tolerance the line needed, if any.
                 */
                adapted?: Adapted | null
                /**
                 * The item's ID.
                 */
                item: number
                /**
                 * The line's index among the item's lines.
                 */
                line: number
              }
            }
          | {
              'item-base': {
                /**
                 * The item's ID.
                 */
                item: number
              }
            }
          | {
              passive: {
                /**
                 * The node's ID.
                 */
                node: number
              }
            }
          | {
              gem: {
                /**
                 * The gem's index in the group.
                 */
                gem: number
                /**
                 * The socket group's index.
                 */
                group: number
              }
            }
          | {
              config: {
                /**
                 * The input's name.
                 */
                input: string
              }
            }
          | 'enemy'
          | {
              rule: string
            }
      }
    }
  | {
      'unhandled-kind': {
        /**
         * The typed resolution's `kind`.
         */
        kind: string
        /**
         * Where it comes from.
         */
        source:
          | 'character'
          | {
              item: {
                /**
                 * The import tolerance the line needed, if any.
                 */
                adapted?: Adapted | null
                /**
                 * The item's ID.
                 */
                item: number
                /**
                 * The line's index among the item's lines.
                 */
                line: number
              }
            }
          | {
              'item-base': {
                /**
                 * The item's ID.
                 */
                item: number
              }
            }
          | {
              passive: {
                /**
                 * The node's ID.
                 */
                node: number
              }
            }
          | {
              gem: {
                /**
                 * The gem's index in the group.
                 */
                gem: number
                /**
                 * The socket group's index.
                 */
                group: number
              }
            }
          | {
              config: {
                /**
                 * The input's name.
                 */
                input: string
              }
            }
          | 'enemy'
          | {
              rule: string
            }
      }
    }
  | {
      'unmodelled-condition': {
        /**
         * The condition's name.
         */
        condition: string
        /**
         * The quantity of the effect left out.
         */
        quantity: string
        /**
         * The source of the effect left out.
         */
        source:
          | 'character'
          | {
              item: {
                /**
                 * The import tolerance the line needed, if any.
                 */
                adapted?: Adapted | null
                /**
                 * The item's ID.
                 */
                item: number
                /**
                 * The line's index among the item's lines.
                 */
                line: number
              }
            }
          | {
              'item-base': {
                /**
                 * The item's ID.
                 */
                item: number
              }
            }
          | {
              passive: {
                /**
                 * The node's ID.
                 */
                node: number
              }
            }
          | {
              gem: {
                /**
                 * The gem's index in the group.
                 */
                gem: number
                /**
                 * The socket group's index.
                 */
                group: number
              }
            }
          | {
              config: {
                /**
                 * The input's name.
                 */
                input: string
              }
            }
          | 'enemy'
          | {
              rule: string
            }
      }
    }
  | {
      'depends-on-not-computed': {
        /**
         * The field it needs.
         */
        field: string
      }
    }
  | 'not-computed'
  | {
      'unmodelled-gem': {
        /**
         * The gem.
         */
        source:
          | 'character'
          | {
              item: {
                /**
                 * The import tolerance the line needed, if any.
                 */
                adapted?: Adapted | null
                /**
                 * The item's ID.
                 */
                item: number
                /**
                 * The line's index among the item's lines.
                 */
                line: number
              }
            }
          | {
              'item-base': {
                /**
                 * The item's ID.
                 */
                item: number
              }
            }
          | {
              passive: {
                /**
                 * The node's ID.
                 */
                node: number
              }
            }
          | {
              gem: {
                /**
                 * The gem's index in the group.
                 */
                gem: number
                /**
                 * The socket group's index.
                 */
                group: number
              }
            }
          | {
              config: {
                /**
                 * The input's name.
                 */
                input: string
              }
            }
          | 'enemy'
          | {
              rule: string
            }
      }
    }
  | {
      'unmodelled-config': {
        /**
         * The input's name.
         */
        input: string
      }
    }
  | {
      'unmodelled-choice': {
        /**
         * Which choice.
         */
        choice: string
        /**
         * Its value, as the build writes it.
         */
        value: string
      }
    }
  | {
      'unmodelled-item': {
        /**
         * The item's ID.
         */
        item: number
        /**
         * Its slot, as the build names it.
         */
        slot: string
        /**
         * What is not modelled.
         */
        what: string
      }
    }
  | {
      'unmodelled-passive': {
        /**
         * The passive.
         */
        source:
          | 'character'
          | {
              item: {
                /**
                 * The import tolerance the line needed, if any.
                 */
                adapted?: Adapted | null
                /**
                 * The item's ID.
                 */
                item: number
                /**
                 * The line's index among the item's lines.
                 */
                line: number
              }
            }
          | {
              'item-base': {
                /**
                 * The item's ID.
                 */
                item: number
              }
            }
          | {
              passive: {
                /**
                 * The node's ID.
                 */
                node: number
              }
            }
          | {
              gem: {
                /**
                 * The gem's index in the group.
                 */
                gem: number
                /**
                 * The socket group's index.
                 */
                group: number
              }
            }
          | {
              config: {
                /**
                 * The input's name.
                 */
                input: string
              }
            }
          | 'enemy'
          | {
              rule: string
            }
        /**
         * What is not modelled.
         */
        what: string
      }
    }
  | {
      'unread-line': {
        /**
         * The line.
         */
        source:
          | 'character'
          | {
              item: {
                /**
                 * The import tolerance the line needed, if any.
                 */
                adapted?: Adapted | null
                /**
                 * The item's ID.
                 */
                item: number
                /**
                 * The line's index among the item's lines.
                 */
                line: number
              }
            }
          | {
              'item-base': {
                /**
                 * The item's ID.
                 */
                item: number
              }
            }
          | {
              passive: {
                /**
                 * The node's ID.
                 */
                node: number
              }
            }
          | {
              gem: {
                /**
                 * The gem's index in the group.
                 */
                gem: number
                /**
                 * The socket group's index.
                 */
                group: number
              }
            }
          | {
              config: {
                /**
                 * The input's name.
                 */
                input: string
              }
            }
          | 'enemy'
          | {
              rule: string
            }
      }
    }
  | 'unchecked-tree-version'
/**
 * A number: a JSON number when finite, otherwise its bit pattern (JSON
 * has no NaN or infinity). Finite values are written in the shortest
 * form that reads back to the same bits.
 */
export type Number = number | NonFinite

/**
 * The document: its format, its version, and the result.
 */
export interface CalcResultDocumentV1 {
  /**
   * Always `loom-calc-result`.
   */
  format: 'loom-calc-result'
  result: CalcResult
  /**
   * Always 1 in this version.
   */
  version: 1
}
/**
 * The result.
 */
export interface CalcResult {
  /**
   * Every field of the game's slice; empty when the game has no
   * calculation.
   */
  fields: FieldResult[]
  /**
   * The build's game.
   */
  game: 'poe1' | 'poe2'
  /**
   * The provenance graph; a node's position is its index.
   */
  graph: Node[]
  reasons: Reasons
  /**
   * The worst state of the fields; `unsupported` when the game has
   * no calculation.
   */
  state: 'complete' | 'partial' | 'unsupported' | 'indeterminate'
}
/**
 * One field's result.
 */
export interface FieldResult {
  /**
   * The field's name (`life`, `resistance.fire`).
   */
  field: string
  /**
   * Why the value cannot be settled: not empty exactly when `state`
   * is `indeterminate`.
   */
  indeterminate?: IndeterminateReason[]
  /**
   * The value's node in `graph`; absent when not computed. Its
   * value is `value`.
   */
  node?: number | null
  /**
   * Why the field is not computed: present exactly when `state` is
   * `unsupported`.
   */
  reason?: UnsupportedReason | null
  /**
   * The state.
   */
  state: 'complete' | 'partial' | 'unsupported' | 'indeterminate'
  /**
   * What was not modelled: not empty when `state` is `partial`,
   * possibly empty when `indeterminate`, empty otherwise.
   */
  unsupported?: UnsupportedReason[]
  /**
   * The value; absent when the field is not computed.
   */
  value?: Number | null
}
/**
 * A non-finite number.
 */
export interface NonFinite {
  /**
   * The bits as 16 lowercase hexadecimal digits.
   */
  f64_bits: string
}
/**
 * A provenance graph node. Its children are earlier nodes; its value
 * is what the engine computed, equal to the graph's evaluation from
 * the leaves.
 */
export interface Node {
  /**
   * The children's indices, in order: a sum's terms, a product's
   * factors, an override's `by` then `replaced`, a minimum's or
   * maximum's two inputs.
   */
  children?: number[]
  /**
   * What the node is.
   */
  kind:
    | {
        contribution: {
          /**
           * How it combines (`base`, `increased`, `more`, `flag`,
           * `override`).
           */
          op: string
          /**
           * What it changes.
           */
          quantity: string
          /**
           * Where it comes from.
           */
          source:
            | 'character'
            | {
                item: {
                  /**
                   * The import tolerance the line needed, if any.
                   */
                  adapted?: Adapted | null
                  /**
                   * The item's ID.
                   */
                  item: number
                  /**
                   * The line's index among the item's lines.
                   */
                  line: number
                }
              }
            | {
                'item-base': {
                  /**
                   * The item's ID.
                   */
                  item: number
                }
              }
            | {
                passive: {
                  /**
                   * The node's ID.
                   */
                  node: number
                }
              }
            | {
                gem: {
                  /**
                   * The gem's index in the group.
                   */
                  gem: number
                  /**
                   * The socket group's index.
                   */
                  group: number
                }
              }
            | {
                config: {
                  /**
                   * The input's name.
                   */
                  input: string
                }
              }
            | 'enemy'
            | {
                rule: string
              }
        }
      }
    | {
        constant: {
          /**
           * What it is.
           */
          label: string
        }
      }
    | 'sum'
    | 'product'
    | 'percent-factor'
    | 'any'
    | 'override'
    | 'floor'
    | 'round-half-up'
    | 'trunc'
    | 'min'
    | 'max'
  /**
   * The breakdown term (`base`, `increased`, `more`, `override`,
   * `result`, or `step.<name>`); absent for leaves and overrides.
   */
  term?: string | null
  /**
   * A number: a JSON number when finite, otherwise its bit pattern (JSON
   * has no NaN or infinity). Finite values are written in the shortest
   * form that reads back to the same bits.
   */
  value: number | NonFinite
}
/**
 * The reasons that apply to every computed field.
 */
export interface Reasons {
  /**
   * Values that could not be settled.
   */
  indeterminate?: IndeterminateReason[]
  /**
   * Content that was not modelled.
   */
  unsupported?: UnsupportedReason[]
}
