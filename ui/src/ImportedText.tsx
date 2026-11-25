// An imported build's text with its colour codes drawn (colourCodes.ts).
import type { ReactNode } from 'react'

import { parts } from './colourCodes'

/** The text with its codes drawn as colour. */
export function ImportedText({ text }: { text: string }): ReactNode {
  return parts(text).map((p, i) =>
    p.colour === null ? (
      <span key={i}>{p.text}</span>
    ) : (
      <span key={i} style={{ color: p.colour }}>
        {p.text}
      </span>
    ),
  )
}
