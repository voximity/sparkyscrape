import { Box, Image } from '@chakra-ui/react';
import { useState } from 'react';
import { getDifficultyString } from '../api/store';

const LevelImage = ({
  difficulty,
  current,
  name,
  fallback,
  message,
}: {
  difficulty: number;
  current?: string;
  name?: string;
  fallback?: boolean;
  message: string;
}) => {
  const [errored, setErrored] = useState(false);

  const box = (
    <Box
      aspectRatio="1 / 1"
      bg="gray.600"
      display="flex"
      alignItems="center"
      justifyContent="center"
      borderRadius="md"
    >
      {errored ? 'Failed to load.' : message}
    </Box>
  );

  if (errored || fallback || (!name && !current)) return box;

  return (
    <Image
      src={
        current
          ? `/levels/${current}.png?d=${Date.now()}`
          : `/levels/${getDifficultyString(difficulty)}/${encodeURIComponent(
              name ?? ''
            )}.png`
      }
      borderRadius="md"
      ignoreFallback
      onError={() => setErrored(true)}
    />
  );
};

export default LevelImage;
