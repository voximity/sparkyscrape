import { Tag } from '@chakra-ui/react';
import { ComponentProps } from 'react';

const DIFFICULTY_COLORS = ['green', 'yellow', 'red', 'purple'];
export const DIFFICULTY_NAMES = ['Easy', 'Medium', 'Hard', 'Legendary'];

const Difficulty = ({
  difficulty,
  ...props
}: { difficulty: number } & ComponentProps<typeof Tag>) => {
  return (
    <Tag colorScheme={DIFFICULTY_COLORS[difficulty] ?? 'green'} {...props}>
      {DIFFICULTY_NAMES[difficulty] ?? 'Easy'}
    </Tag>
  );
};

export default Difficulty;
