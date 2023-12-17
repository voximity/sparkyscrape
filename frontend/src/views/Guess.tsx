import {
  Button,
  Container,
  Flex,
  FormControl,
  FormHelperText,
  FormLabel,
  Grid,
  GridItem,
  HStack,
  Heading,
  Radio,
  RadioGroup,
  Stack,
  Text,
} from '@chakra-ui/react';
import { ChangeEvent, useState } from 'react';
import { DIFFICULTY_STRINGS, getDifficultyString } from '../api/store';
import Difficulty, { DIFFICULTY_NAMES } from '../components/Difficulty';
import LevelImage from '../components/LevelImage';
import LevelName from '../components/LevelName';

const Guess = () => {
  const [loading, setLoading] = useState(false);
  const [difficulty, setDifficulty] = useState('1');
  const [guess, setGuess] = useState<{ level: string; distance: number }>();
  const [file, setFile] = useState<File | null>();
  const [imageData, setImageData] = useState('');

  const onFileChange = (e: ChangeEvent<HTMLInputElement>) => {
    const f = e.target.files?.[0];
    if (f) {
      setFile(f);
      const reader = new FileReader();
      reader.onload = (ev) => {
        setImageData(ev.target!.result as string);
      };
      reader.readAsDataURL(f);
    }
  };

  const onSearch = async () => {
    if (loading) return;
    if (!file) return;

    setLoading(true);
    const body = new FormData();
    body.append('data', file);

    fetch(`/api/guess/${getDifficultyString(Number(difficulty))}`, {
      method: 'POST',
      body,
    })
      .then(async (res) => {
        setGuess(await res.json());
      })
      .finally(() => {
        setLoading(false);
      });
  };

  return (
    <Container maxW="container.lg" my="3">
      <Stack spacing="3">
        <Heading>Guess</Heading>
        <FormControl>
          <FormLabel>Image upload</FormLabel>
          <input type="file" onChange={onFileChange} accept="image/png" />
        </FormControl>
        <FormControl>
          <FormLabel>Level difficulty</FormLabel>
          <RadioGroup value={difficulty} onChange={setDifficulty}>
            <HStack spacing="24px">
              {DIFFICULTY_STRINGS.map((s, i) => (
                <Radio value={i.toString()}>{DIFFICULTY_NAMES[i]}</Radio>
              ))}
            </HStack>
          </RadioGroup>
          <FormHelperText>
            Controls what database the search will be made in.
          </FormHelperText>
        </FormControl>
        <Button
          colorScheme="blue"
          isLoading={loading}
          isDisabled={!file}
          onClick={onSearch}
        >
          Guess
        </Button>
        <Grid templateColumns="1fr 1fr" gap="4">
          <GridItem>
            <Flex
              direction="row"
              alignItems="center"
              justifyContent="space-between"
            >
              <Heading size="md">Guessing</Heading>
              <Difficulty difficulty={Number(difficulty)} />
            </Flex>
          </GridItem>
          <GridItem>
            <Flex
              direction="row"
              alignItems="center"
              justifyContent="space-between"
            >
              <Heading size="md">My guess</Heading>
              {guess !== undefined && <Text>{guess.distance.toFixed(2)}</Text>}
            </Flex>
          </GridItem>
          <GridItem>
            <LevelImage
              src={imageData || undefined}
              fallback={!imageData}
              message="Select an image from above."
            />
          </GridItem>
          <GridItem>
            <LevelImage
              difficulty={Number(difficulty)}
              name={guess?.level}
              fallback={!guess}
              message={
                loading ? 'Making a guess...' : 'Click Guess to make a guess.'
              }
            />
          </GridItem>
          <GridItem />
          <GridItem display="flex" justifyContent="center">
            {guess && <LevelName name={guess.level} copyable />}
          </GridItem>
        </Grid>
      </Stack>
    </Container>
  );
};

export default Guess;
