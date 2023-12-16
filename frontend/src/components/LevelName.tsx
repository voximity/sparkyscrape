import { Code } from '@chakra-ui/react';
import { ComponentProps } from 'react';

const LevelName = ({
  name,
  color,
  size,
}: {
  name: string;
  size?: ComponentProps<typeof Code>['fontSize'];
  color?: ComponentProps<typeof Code>['colorScheme'];
}) => (
  <Code
    fontSize={size ?? '2xl'}
    fontWeight="bold"
    borderRadius="md"
    px="3"
    colorScheme={color}
  >
    {name}
  </Code>
);

export default LevelName;
