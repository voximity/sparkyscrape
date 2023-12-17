import { ChakraProvider, extendTheme } from '@chakra-ui/react';
import { useEffect } from 'react';
import { BrowserRouter, Outlet, Route, Routes } from 'react-router-dom';
import Header from './components/Header';
import { socket } from './socket';
import Home from './views/Home';
import { Provider, useDispatch } from 'react-redux';
import store, { setConnected } from './api/store';
import {
  setChannel,
  setChannelGame,
  setChannelGameData,
  setChannelGameTimeout,
  setChannelGameWin,
} from './api/channels';
import Channel from './views/Channel';
import Guess from './views/Guess';

export { Link as ReactRouterLink } from 'react-router-dom';

const theme = extendTheme({
  config: {
    initialColorMode: 'dark',
    useSystemColorMode: false,
  },
});

const Layout = () => {
  const dispatch = useDispatch();

  useEffect(() => {
    function onHello(data: { channels: Record<string, string> }) {
      console.log('received hello');

      for (const [id, name] of Object.entries(data.channels)) {
        dispatch(setChannel({ id, name }));
      }

      dispatch(setConnected(true));
    }

    function onGuessStart({
      channel_id: id,
      difficulty,
    }: {
      channel_id: string;
      difficulty: number;
    }) {
      dispatch(setChannelGame({ id, game: { downloaded: false, difficulty } }));
    }

    function onGuessData({
      channel_id: id,
      guess,
    }: {
      channel_id: string;
      guess?: [string, number];
    }) {
      dispatch(
        setChannelGameData({
          id,
          guess: guess?.[0],
          distance: guess?.[1],
        })
      );
    }

    function onGuessWin({
      channel_id: id,
      answer,
      incorrect,
    }: {
      channel_id: string;
      answer: string;
      incorrect: boolean;
    }) {
      dispatch(setChannelGameWin({ id, answer, incorrect }));
    }

    function onGuessTimeout({ channel_id: id }: { channel_id: string }) {
      dispatch(setChannelGameTimeout({ id }));
    }

    function onDisconnect() {
      dispatch(setConnected(false));
    }

    socket.on('hello', onHello);
    socket.on('guess/start', onGuessStart);
    socket.on('guess/data', onGuessData);
    socket.on('guess/win', onGuessWin);
    socket.on('guess/timeout', onGuessTimeout);
    socket.on('disconnect', onDisconnect);

    return () => {
      socket.off('hello', onHello);
      socket.off('guess/start', onGuessStart);
      socket.off('guess/data', onGuessData);
      socket.off('guess/win', onGuessWin);
      socket.off('guess/timeout', onGuessTimeout);
      socket.off('disconnect', onDisconnect);
    };
  }, [dispatch]);

  return (
    <>
      <Header />
      <Outlet />
    </>
  );
};

function App() {
  return (
    <Provider store={store}>
      <ChakraProvider theme={theme}>
        <BrowserRouter>
          <Routes>
            <Route path="/" element={<Layout />}>
              <Route index element={<Home />} />
              <Route path="channel/:id" element={<Channel />} />
              <Route path="guess" element={<Guess />} />
            </Route>
          </Routes>
        </BrowserRouter>
      </ChakraProvider>
    </Provider>
  );
}

export default App;
