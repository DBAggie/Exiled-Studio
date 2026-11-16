// The import workflow as an explicit state machine (ADR-004: targeted
// state machines for multi-step workflows):
//
//   idle --started--> reading --done--> review --dismissed--> idle
//                        |--failed--> failed --started--> reading
//                        |--cancelled--> idle
//
// An event that does not apply in the current state is ignored.
import { createSlice, type PayloadAction } from '@reduxjs/toolkit'

export type ImportState =
  | { state: 'idle' }
  | { state: 'reading' }
  | { state: 'review' }
  | { state: 'failed'; code: string }

export const importFlow = createSlice({
  name: 'importFlow',
  initialState: { state: 'idle' } as ImportState,
  reducers: {
    importStarted(s) {
      return s.state === 'reading' ? s : { state: 'reading' }
    },
    importDone(s) {
      return s.state === 'reading' ? { state: 'review' } : s
    },
    importFailed(s, action: PayloadAction<string>) {
      return s.state === 'reading'
        ? { state: 'failed', code: action.payload }
        : s
    },
    importCancelled(s) {
      return s.state === 'reading' ? { state: 'idle' } : s
    },
    importDismissed(s) {
      return s.state === 'review' || s.state === 'failed'
        ? { state: 'idle' }
        : s
    },
  },
})

export const importActions = importFlow.actions
