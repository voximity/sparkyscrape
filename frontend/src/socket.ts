import { io } from 'socket.io-client';

export const socket =
  process.env.NODE_ENV === 'development'
    ? io('http://127.0.0.1:3579', { upgrade: false, transports: ['websocket'] })
    : io({ upgrade: false, transports: ['websocket'] });
