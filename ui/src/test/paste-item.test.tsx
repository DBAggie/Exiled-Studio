// B-2 item evidence, E3: "Paste an item" on the Equipment page. A game
// copy pasted on the page opens the panel with the text; Add makes it an
// item in the slot picked, as one undoable step; a text that is not an
// item shows the error and adds nothing.
import { fireEvent, screen, waitFor } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { expectAccessible, renderApp } from './render'

const COPY = [
  'Item Class: Rings',
  'Rarity: Rare',
  'Pasted Band',
  'Plain Test Ring',
  '--------',
  'Item Level: 50',
  '--------',
  '+20 to maximum Life',
].join('\n')

async function equipment() {
  const app = await renderApp()
  await app.user.type(
    screen.getByLabelText('Paste a share code or build XML'),
    '<PathOfBuilding/>',
  )
  await app.user.click(screen.getByRole('button', { name: 'Import build' }))
  await screen.findByRole('heading', { name: 'Imported build', level: 1 })
  await app.user.click(screen.getByRole('button', { name: 'Equipment' }))
  await screen.findByRole('heading', { name: 'Equipment', level: 1 })
  return app
}

describe('paste an item (B-2 item evidence)', () => {
  it('opens on a page paste and adds the item to the slot picked', async () => {
    const { user, api } = await equipment()
    // A paste outside any field opens the panel with the text.
    fireEvent.paste(document.body, {
      clipboardData: { getData: () => COPY },
    })
    const text = (await screen.findByLabelText(
      'Item text',
    )) as HTMLTextAreaElement
    expect(text.value).toBe(COPY)
    await user.selectOptions(screen.getByLabelText('Put it in'), 'Ring 2')
    await expectAccessible()
    await user.click(screen.getByRole('button', { name: 'Add the item' }))
    expect(
      await screen.findByText('Added Pasted Band.', { exact: false }),
    ).toBeTruthy()
    const build = (await api.snapshot()).build
    const added = build?.items.find((i) => i.name === 'Pasted Band')
    expect(added?.slot).toBe('Ring 2')
    expect((await api.snapshot()).history.undo?.kind).toBe('item-added')
  })

  it('leaves a paste into a field alone and shows why a text is refused', async () => {
    const { user, api } = await equipment()
    await user.click(screen.getByText('Paste an item'))
    const text = screen.getByLabelText('Item text') as HTMLTextAreaElement
    const before = (await api.snapshot()).build?.items.length
    await user.type(text, 'not an item')
    await user.click(screen.getByRole('button', { name: 'Add the item' }))
    await waitFor(() =>
      expect(
        screen.getByText(/could not be added: it is not an item/),
      ).toBeTruthy(),
    )
    expect((await api.snapshot()).build?.items.length).toBe(before)
  })
})
