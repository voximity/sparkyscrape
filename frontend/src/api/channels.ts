import { PayloadAction, createSlice } from '@reduxjs/toolkit';
import { RootState } from './store';

export type ChannelState = {
  id: string;
  name: string;
  past_games: ChannelStateGame[];
  game?: ChannelStateGame;
};

export type ChannelStateGame = {
  difficulty: number;
  downloaded: boolean;
  guess?: string;
  distance?: number;
  result?: ChannelStateGameResult;
};

export type ChannelStateGameResult =
  | {
      type: 'timeout';
    }
  | { type: 'win'; answer: string; incorrect: boolean };

const MAX_PAST_GAMES = 25;

const slice = createSlice({
  name: 'channels',
  initialState: {} as Record<string, ChannelState>,
  reducers: {
    setChannel: (
      state,
      action: PayloadAction<{ id: string; name: string }>
    ) => {
      state[action.payload.id] = { ...action.payload, past_games: [] };
    },

    setChannelGame: (
      state,
      action: PayloadAction<{ id: string; game?: ChannelStateGame }>
    ) => {
      const cur = state[action.payload.id].game;
      const past_games = state[action.payload.id].past_games;
      if (cur) {
        past_games.unshift(cur);
        if (past_games.length >= MAX_PAST_GAMES) {
          past_games.pop();
        }
      }

      state[action.payload.id].game = action.payload.game;
    },

    setChannelGameData: (
      state,
      action: PayloadAction<{
        id: string;
        guess?: string;
        distance?: number;
      }>
    ) => {
      const game = state[action.payload.id]?.game;
      if (game) {
        game.downloaded = true;
        game.guess = action.payload.guess;
        game.distance = action.payload.distance;
      }
    },

    setChannelGameWin: (
      state,
      {
        payload: { id, answer, incorrect },
      }: PayloadAction<{ id: string; answer: string; incorrect: boolean }>
    ) => {
      const game = state[id]?.game;
      if (game) {
        game.result = { type: 'win', answer, incorrect };
      }
    },

    setChannelGameTimeout: (state, action: PayloadAction<{ id: string }>) => {
      const game = state[action.payload.id]?.game;
      if (game) {
        game.result = { type: 'timeout' };
      }
    },
  },
});

export const selectChannels = (state: RootState) => state.channels;

export const selectChannelName = (id: string) => (state: RootState) =>
  state.channels[id]?.name;

export const selectChannelGame = (id: string) => (state: RootState) =>
  state.channels[id]?.game;

export const selectChannelGameDifficulty = (id: string) => (state: RootState) =>
  state.channels[id]?.game?.difficulty;

export const selectChannelGameDownloaded = (id: string) => (state: RootState) =>
  state.channels[id]?.game?.downloaded;

export const selectChannelGameGuess = (id: string) => (state: RootState) =>
  state.channels[id]?.game?.guess;

export const selectChannelGameGuessDistance =
  (id: string) => (state: RootState) =>
    state.channels[id]?.game?.distance;

export const selectChannelGameResult = (id: string) => (state: RootState) =>
  state.channels[id]?.game?.result;

export const selectChannelPastGames = (id: string) => (state: RootState) =>
  state.channels[id]?.past_games;

export const {
  setChannel,
  setChannelGame,
  setChannelGameData,
  setChannelGameWin,
  setChannelGameTimeout,
} = slice.actions;

export default slice;
