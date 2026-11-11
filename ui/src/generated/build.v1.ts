// Generated from schemas/build.v1.schema.json by pnpm generate:schema; do not edit.

/**
 * A node of untyped content.
 */
export type OpaqueNode =
  | {
      element: {
        /**
         * Attributes, in source order.
         */
        attributes?: Attribute[]
        /**
         * Children, in source order.
         */
        children?: OpaqueNode[]
        /**
         * Name.
         */
        name: string
      }
    }
  | {
      text: string
    }
  | {
      comment: string
    }
  | {
      'processing-instruction': {
        /**
         * Data, if any.
         */
        data?: string | null
        /**
         * Target.
         */
        target: string
      }
    }
/**
 * One entry of a list with untyped content in place.
 */
export type ConfigSetEntry =
  | {
      typed: ConfigSet
    }
  | {
      opaque: OpaqueNode
    }
/**
 * One entry of a list with untyped content in place.
 */
export type ConfigInputEntry =
  | {
      typed: ConfigInput
    }
  | {
      opaque: OpaqueNode
    }
/**
 * One entry of a list with untyped content in place.
 */
export type ItemsEntryEntry =
  | {
      typed: ItemsEntry
    }
  | {
      opaque: OpaqueNode
    }
/**
 * An item or an item set.
 */
export type ItemsEntry =
  | {
      item: Item
    }
  | {
      set: ItemSet
    }
/**
 * A header line.
 */
export type HeaderLine = 'item-class' | 'rarity' | 'separator' | 'name' | 'base'
/**
 * Why a modifier line is not resolved by design.
 */
export type Exclusion = 'not-selected' | 'custom' | 'section'
/**
 * An import tolerance a line needed.
 */
export type Adapted = 'case-folded' | 'split-section'
/**
 * A typed result read from game data.
 */
export type TypedResolution =
  | {
      'granted-skill': {
        /**
         * The gem.
         */
        gem: string
        /**
         * The level.
         */
        level: number
        /**
         * The data row that grants it.
         */
        source: string
      }
    }
  | {
      'item-property': {
        /**
         * The property's data ID.
         */
        property: string
        /**
         * Values by placeholder, in text order.
         */
        values: PropertyValue[]
      }
    }
  | {
      'catalyst-quality': {
        /**
         * The quality as shown.
         */
        quality: string
        /**
         * The quality type's data ID.
         */
        quality_type: string
        /**
         * Its stats.
         */
        stats: StatLine[]
      }
    }
/**
 * Why a line did not resolve.
 */
export type UnresolvedReason =
  | 'no-template'
  | 'no-variant-fits'
  | 'only-shadowed'
  | {
      ambiguous: string[][]
    }
  | {
      'no-base': NoBase
    }
/**
 * Why an item has no base.
 */
export type NoBase =
  | {
      unnamed: string
    }
  | 'not-found'
  | 'scope-conflict'
/**
 * Item rarity.
 */
export type Rarity = 'normal' | 'magic' | 'rare' | 'unique' | 'relic'
/**
 * One entry of a list with untyped content in place.
 */
export type ItemPartEntry =
  | {
      typed: ItemPart
    }
  | {
      opaque: OpaqueNode
    }
/**
 * Where an item's body sits among untyped children.
 */
export type ItemPart = 'body'
/**
 * One entry of a list with untyped content in place.
 */
export type SlotEntryEntry =
  | {
      typed: SlotEntry
    }
  | {
      opaque: OpaqueNode
    }
/**
 * One entry of a list with untyped content in place.
 */
export type BuildPartEntry =
  | {
      typed: BuildPart
    }
  | {
      opaque: OpaqueNode
    }
/**
 * Where a typed section sits among untyped ones.
 */
export type BuildPart = 'character' | 'passives' | 'skills' | 'items' | 'config'
/**
 * One entry of a list with untyped content in place.
 */
export type PassiveSpecEntry =
  | {
      typed: PassiveSpec
    }
  | {
      opaque: OpaqueNode
    }
/**
 * One entry of a list with untyped content in place.
 */
export type JewelSocketEntry =
  | {
      typed: JewelSocket
    }
  | {
      opaque: OpaqueNode
    }
/**
 * One entry of a list with untyped content in place.
 */
export type SpecPartEntry =
  | {
      typed: SpecPart
    }
  | {
      opaque: OpaqueNode
    }
/**
 * Where a spec's typed child sits among untyped ones.
 */
export type SpecPart = 'jewel-sockets'
/**
 * One entry of a list with untyped content in place.
 */
export type SkillSetEntry =
  | {
      typed: SkillSet
    }
  | {
      opaque: OpaqueNode
    }
/**
 * One entry of a list with untyped content in place.
 */
export type SocketGroupEntry =
  | {
      typed: SocketGroup
    }
  | {
      opaque: OpaqueNode
    }
/**
 * One entry of a list with untyped content in place.
 */
export type GemEntry =
  | {
      typed: Gem
    }
  | {
      opaque: OpaqueNode
    }

/**
 * The document: its format, its version, and the build.
 */
export interface BuildDocumentV1 {
  build: Build
  /**
   * Always `loom-build`.
   */
  format: 'loom-build'
  /**
   * Always 1 in this version.
   */
  version: 1
}
/**
 * The build.
 */
export interface Build {
  /**
   * Untyped attributes of the build's container.
   */
  attributes?: Attribute[]
  character: Character
  config: Config
  /**
   * The data bundle the build was made with, if recorded.
   */
  data?: DataVersion | null
  /**
   * Untyped content after the build's container.
   */
  epilog?: OpaqueNode[]
  /**
   * The game.
   */
  game: 'poe1' | 'poe2'
  /**
   * The game-specific choices; of the build's game.
   */
  game_specific:
    | {
        poe1: Poe1Choices
      }
    | {
        poe2: Poe2Choices
      }
  items: Items
  /**
   * The sections in order, typed ones as markers.
   */
  layout: BuildPartEntry[]
  passives: Passives
  /**
   * Untyped content before the build's container.
   */
  prolog?: OpaqueNode[]
  skills: Skills
}
/**
 * One untyped attribute.
 */
export interface Attribute {
  /**
   * Name.
   */
  name: string
  /**
   * Value.
   */
  value: string
}
/**
 * The character.
 */
export interface Character {
  /**
   * The ascendancy, if set.
   */
  ascendancy?: string | null
  /**
   * Untyped attributes.
   */
  attributes?: Attribute[]
  /**
   * Untyped children.
   */
  children?: OpaqueNode[]
  /**
   * The class, if set.
   */
  class?: string | null
  /**
   * The level, if set.
   */
  level?: SourcedUint32 | null
  /**
   * The main socket group (1-based), if set.
   */
  main_socket_group?: SourcedUint32 | null
}
/**
 * A typed value and the text it was read from, if any.
 */
export interface SourcedUint32 {
  /**
   * The source text, while the value is unchanged.
   */
  source?: string | null
  /**
   * The value.
   */
  value: number
}
/**
 * Config.
 */
export interface Config {
  /**
   * The active set, if set.
   */
  active?: number | null
  /**
   * Untyped attributes.
   */
  attributes?: Attribute[]
  /**
   * The sets.
   */
  sets: ConfigSetEntry[]
}
/**
 * One config set.
 */
export interface ConfigSet {
  /**
   * Untyped attributes.
   */
  attributes?: Attribute[]
  /**
   * The set's ID.
   */
  id: number
  /**
   * Inputs.
   */
  inputs: ConfigInputEntry[]
  /**
   * The title, if any.
   */
  title?: string | null
}
/**
 * One config input.
 */
export interface ConfigInput {
  /**
   * Untyped attributes.
   */
  attributes?: Attribute[]
  /**
   * The key.
   */
  name: string
  /**
   * The value.
   */
  value:
    | {
        boolean: boolean
      }
    | {
        number: Sourced_Number
      }
    | {
        string: string
      }
}
/**
 * A typed value and the text it was read from, if any.
 */
export interface Sourced_Number {
  /**
   * The source text, while the value is unchanged.
   */
  source?: string | null
  /**
   * The value.
   */
  value: number | NonFinite
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
 * The data bundle a build was made with.
 */
export interface DataVersion {
  /**
   * The bundle's game.
   */
  game: 'poe1' | 'poe2'
  /**
   * The bundle's data schema version.
   */
  schema_version: number
  /**
   * The Steam build, if known.
   */
  steam_build?: number | null
}
/**
 * Path of Exile choices, as the game data names them.
 */
export interface Poe1Choices {
  /**
   * The bandit choice, if any.
   */
  bandit?: string | null
  /**
   * The major pantheon god, if any.
   */
  pantheon_major?: string | null
  /**
   * The minor pantheon god, if any.
   */
  pantheon_minor?: string | null
}
/**
 * Path of Exile 2 choices (none typed yet).
 */
export interface Poe2Choices {}
/**
 * Items and item sets.
 */
export interface Items {
  /**
   * The active item set, if set.
   */
  active_set?: number | null
  /**
   * Untyped attributes.
   */
  attributes?: Attribute[]
  /**
   * Items and sets in order.
   */
  entries: ItemsEntryEntry[]
}
/**
 * One item.
 */
export interface Item {
  /**
   * Untyped attributes.
   */
  attributes?: Attribute[]
  body: ItemBody
  /**
   * The item's ID.
   */
  id: number
  /**
   * The item's children.
   */
  layout: ItemPartEntry[]
  /**
   * Legacy-XML whitespace of the body's text (`lines + 1` entries),
   * or empty.
   */
  text_spacing?: string[]
}
/**
 * The item as read from its text.
 */
export interface ItemBody {
  /**
   * The base name, if written.
   */
  base?: string | null
  /**
   * The implicit count, if written.
   */
  implicits?: number | null
  /**
   * The item class, if written.
   */
  item_class?: string | null
  /**
   * Every source line, in order.
   */
  lines: ItemLine[]
  /**
   * The name, if any.
   */
  name?: string | null
  /**
   * Rarity, if read.
   */
  rarity?: Rarity | null
  variants: VariantSelection
}
/**
 * One source line of an item.
 */
export interface ItemLine {
  /**
   * What it is.
   */
  kind:
    | {
        header: HeaderLine
      }
    | {
        spec: {
          /**
           * The key.
           */
          key: string
          /**
           * Whether the import adapter knows the key.
           */
          known: boolean
          /**
           * The value, as written.
           */
          value: string
        }
      }
    | {
        'variant-base': {
          /**
           * The base text.
           */
          base: string
          /**
           * Whether the variant selection includes it.
           */
          selected: boolean
        }
      }
    | {
        influence: string
      }
    | 'flag'
    | {
        modifier: ModifierLine
      }
    | 'unrecognized'
  /**
   * The line as written.
   */
  source: string
}
/**
 * A modifier line.
 */
export interface ModifierLine {
  /**
   * Prefixes, in line order.
   */
  prefixes?: Prefix[]
  /**
   * What the line resolved to.
   */
  resolution:
    | 'pending'
    | {
        excluded: Exclusion
      }
    | {
        resolved: {
          /**
           * The tolerance it needed, if any.
           */
          adapted?: Adapted | null
          /**
           * The result.
           */
          result:
            | {
                stats: StatLine[]
              }
            | {
                typed: TypedResolution
              }
        }
      }
    | {
        continues: {
          /**
           * Index of that line among the item's lines.
           */
          line: number
        }
      }
    | {
        unresolved: UnresolvedReason
      }
  /**
   * The section.
   */
  section:
    | 'enchant'
    | 'implicit'
    | 'explicit'
    | {
        other: string
      }
  /**
   * Whether the variant selection includes the line.
   */
  selected: boolean
  /**
   * The text without prefixes.
   */
  text: string
}
/**
 * A prefix or flag word on a modifier line.
 */
export interface Prefix {
  /**
   * The key.
   */
  key: string
  /**
   * The value, if any.
   */
  value?: string | null
}
/**
 * A stat ID and its value.
 */
export interface StatLine {
  /**
   * The stat.
   */
  stat: string
  /**
   * Its value.
   */
  value:
    | {
        exact: {
          /**
           * Highest raw value.
           */
          max: number
          /**
           * Lowest raw value.
           */
          min: number
        }
      }
    | {
        ranged: {
          /**
           * Highest raw value.
           */
          max: number
          /**
           * Lowest raw value.
           */
          min: number
        }
      }
    | {
        hidden: {
          /**
           * The assumed value, if clear.
           */
          assumed?: number | null
        }
      }
    | {
        lookup: number[]
      }
}
/**
 * One placeholder value of a property line.
 */
export interface PropertyValue {
  /**
   * The placeholder's number.
   */
  slot: number
  /**
   * The value's text.
   */
  text: string
}
/**
 * Variant selection.
 */
export interface VariantSelection {
  /**
   * Alternate variants.
   */
  alternates?: AltVariant[]
  /**
   * Number of variants.
   */
  count: number
  /**
   * The selected variant, if written.
   */
  selected?: number | null
}
/**
 * An alternate variant.
 */
export interface AltVariant {
  /**
   * Which alternate (0 for the first).
   */
  index: number
  /**
   * The selected variant, if written.
   */
  selected?: number | null
}
/**
 * An item set.
 */
export interface ItemSet {
  /**
   * Untyped attributes.
   */
  attributes?: Attribute[]
  /**
   * The set's ID.
   */
  id: number
  /**
   * Slots.
   */
  slots: SlotEntryEntry[]
  /**
   * The title, if any.
   */
  title?: string | null
}
/**
 * One slot of an item set.
 */
export interface SlotEntry {
  /**
   * Untyped attributes.
   */
  attributes?: Attribute[]
  /**
   * The item, if any.
   */
  item?: number | null
  /**
   * The slot.
   */
  slot: string
}
/**
 * Passive specs.
 */
export interface Passives {
  /**
   * The active spec, if set.
   */
  active?: number | null
  /**
   * Untyped attributes.
   */
  attributes?: Attribute[]
  /**
   * The specs.
   */
  specs: PassiveSpecEntry[]
}
/**
 * One passive tree spec.
 */
export interface PassiveSpec {
  /**
   * Untyped attributes.
   */
  attributes?: Attribute[]
  /**
   * The spec's ID.
   */
  id: number
  /**
   * Jewel sockets.
   */
  jewel_sockets: JewelSocketEntry[]
  /**
   * The spec's children.
   */
  layout: SpecPartEntry[]
  mastery_effects: Sourced_ArrayOf_MasteryEffect
  nodes: Sourced_ArrayOfUint32
  /**
   * The title, if any.
   */
  title?: string | null
  /**
   * The tree version, if any.
   */
  tree_version?: string | null
}
/**
 * A jewel socket and its item.
 */
export interface JewelSocket {
  /**
   * Untyped attributes.
   */
  attributes?: Attribute[]
  /**
   * The item, if any.
   */
  item?: number | null
  /**
   * The socket node.
   */
  node: number
}
/**
 * Mastery effects, in order.
 */
export interface Sourced_ArrayOf_MasteryEffect {
  /**
   * The source text, while the value is unchanged.
   */
  source?: string | null
  /**
   * The value.
   */
  value: MasteryEffect[]
}
/**
 * A mastery node and its chosen effect.
 */
export interface MasteryEffect {
  /**
   * The effect.
   */
  effect: number
  /**
   * The node.
   */
  node: number
}
/**
 * Allocated nodes, in order.
 */
export interface Sourced_ArrayOfUint32 {
  /**
   * The source text, while the value is unchanged.
   */
  source?: string | null
  /**
   * The value.
   */
  value: number[]
}
/**
 * Skills.
 */
export interface Skills {
  /**
   * The active set, if set.
   */
  active?: number | null
  /**
   * Untyped attributes.
   */
  attributes?: Attribute[]
  /**
   * The sets.
   */
  sets: SkillSetEntry[]
}
/**
 * One skill set.
 */
export interface SkillSet {
  /**
   * Untyped attributes.
   */
  attributes?: Attribute[]
  /**
   * Socket groups.
   */
  groups: SocketGroupEntry[]
  /**
   * The set's ID.
   */
  id: number
  /**
   * The title, if any.
   */
  title?: string | null
}
/**
 * A group of linked gems.
 */
export interface SocketGroup {
  /**
   * Untyped attributes.
   */
  attributes?: Attribute[]
  /**
   * Whether the group is enabled.
   */
  enabled: boolean
  /**
   * Gems.
   */
  gems: GemEntry[]
  /**
   * The label, if any.
   */
  label?: string | null
  /**
   * The main active skill (1-based), if set.
   */
  main_active_skill?: SourcedUint32 | null
  /**
   * The slot, if any.
   */
  slot?: string | null
}
/**
 * One gem.
 */
export interface Gem {
  /**
   * Untyped attributes.
   */
  attributes?: Attribute[]
  /**
   * Whether the gem is enabled.
   */
  enabled: boolean
  /**
   * The gem item, if known.
   */
  gem?: string | null
  level: SourcedUint321
  /**
   * The name as written, if any.
   */
  name?: string | null
  quality: SourcedInt32
  /**
   * The skill, if known.
   */
  skill?: string | null
}
/**
 * A typed value and the text it was read from, if any.
 */
export interface SourcedUint321 {
  /**
   * The source text, while the value is unchanged.
   */
  source?: string | null
  /**
   * The value.
   */
  value: number
}
/**
 * Quality.
 */
export interface SourcedInt32 {
  /**
   * The source text, while the value is unchanged.
   */
  source?: string | null
  /**
   * The value.
   */
  value: number
}
