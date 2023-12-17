import {
  Center,
  Container,
  Flex,
  Grid,
  GridItem,
  Heading,
  Spinner,
  Stack,
  Text,
} from '@chakra-ui/react';
import { useEffect, useState } from 'react';
import Confetti from 'react-confetti';
import { useSelector } from 'react-redux';
import { useParams } from 'react-router-dom';
import { useScrollbarWidth, useWindowSize } from 'react-use';
import {
  selectChannelGameDifficulty,
  selectChannelGameDownloaded,
  selectChannelGameGuess,
  selectChannelGameGuessDistance,
  selectChannelGameResult,
  selectChannelName,
} from '../api/channels';
import LevelName from '../components/LevelName';
import LevelImage from '../components/LevelImage';
import Difficulty from '../components/Difficulty';
import PastGames from '../components/PastGames';
import { createPortal } from 'react-dom';

const Channel = () => {
  const { id } = useParams();
  const { width, height } = useWindowSize();
  const scrollbar = useScrollbarWidth();

  const name = useSelector(selectChannelName(id!));
  const difficulty = useSelector(selectChannelGameDifficulty(id!));
  const downloaded = useSelector(selectChannelGameDownloaded(id!));
  const guess = useSelector(selectChannelGameGuess(id!));
  const distance = useSelector(selectChannelGameGuessDistance(id!));
  const result = useSelector(selectChannelGameResult(id!));

  const [correct, setCorrect] = useState<number | undefined>(undefined);
  useEffect(() => {
    if (result?.type === 'win' && guess === result.answer) {
      setCorrect((n) => 1 - (n ?? 0));
    }
  }, [result, guess]);

  if (!name)
    return (
      <Center>
        <Spinner />
      </Center>
    );

  return (
    <Container maxW="container.lg" my="3">
      {correct !== undefined &&
        createPortal(
          <Confetti
            width={width - (scrollbar ?? 0)}
            height={height}
            recycle={false}
          />,
          document.getElementById('confetti-container')!,
          correct.toString()
        )}
      <Stack spacing="3">
        <Heading>{name}</Heading>
        {difficulty !== undefined && (
          <Grid templateColumns="1fr 1fr" gap="4">
            <GridItem>
              <Flex
                direction="row"
                alignItems="center"
                justifyContent="space-between"
              >
                <Heading size="md">Guessing</Heading>
                <Difficulty difficulty={difficulty} />
              </Flex>
            </GridItem>
            <GridItem>
              <Flex
                direction="row"
                alignItems="center"
                justifyContent="space-between"
              >
                <Heading size="md">My guess</Heading>
                {distance !== undefined && <Text>{distance.toFixed(2)}</Text>}
              </Flex>
            </GridItem>
            <GridItem>
              <LevelImage
                difficulty={difficulty}
                current={id}
                fallback={!downloaded}
                message="Downloading..."
              />
            </GridItem>
            <GridItem>
              <LevelImage
                key={guess}
                difficulty={difficulty}
                name={guess}
                fallback={!downloaded}
                message="Making a guess..."
              />
            </GridItem>
            <GridItem display="flex" justifyContent="center">
              {result?.type === 'timeout' ? (
                <Text>Timed out, no answer available</Text>
              ) : result?.type === 'win' ? (
                <LevelName
                  name={result.answer}
                  color={
                    result?.type === 'win'
                      ? result.answer === guess
                        ? 'green'
                        : result.incorrect
                        ? 'red'
                        : 'blue'
                      : undefined
                  }
                />
              ) : (
                <Text>Waiting for an answer...</Text>
              )}
            </GridItem>
            <GridItem display="flex" justifyContent="center">
              {guess ? (
                <LevelName name={guess} copyable />
              ) : (
                <Text>No guess available</Text>
              )}
            </GridItem>
          </Grid>
        )}
        <Heading>Past games</Heading>
        <PastGames id={id!} />
      </Stack>
    </Container>
  );
};

export default Channel;
