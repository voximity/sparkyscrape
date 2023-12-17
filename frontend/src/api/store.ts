import {
  PayloadAction,
  combineReducers,
  configureStore,
  createSlice,
} from '@reduxjs/toolkit';
import channels from './channels';

export const DIFFICULTY_STRINGS = ['easy', 'medium', 'hard', 'legendary'];
export function getDifficultyString(difficulty: number): string {
  return DIFFICULTY_STRINGS[difficulty] ?? 'easy';
}

const infoSlice = createSlice({
  name: 'info',
  initialState: { connected: false },
  reducers: {
    setConnected: (state, action: PayloadAction<boolean>) => {
      state.connected = action.payload;
    },
  },
});

export const selectConnected = (state: RootState) => state.info.connected;

export const { setConnected } = infoSlice.actions;

const store = configureStore({
  reducer: combineReducers({
    info: infoSlice.reducer,
    channels: channels.reducer,
  }),
});

export type RootState = ReturnType<typeof store.getState>;
export default store;
