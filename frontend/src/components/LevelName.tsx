import { CopyIcon } from '@chakra-ui/icons';
import { Box, Code } from '@chakra-ui/react';
import { ComponentProps } from 'react';

const LevelName = ({
  name,
  color,
  size,
  copyable,
}: {
  name: string;
  size?: ComponentProps<typeof Code>['fontSize'];
  color?: ComponentProps<typeof Code>['colorScheme'];
  copyable?: boolean;
}) => (
  <Code
    fontSize={size ?? '2xl'}
    fontWeight="bold"
    borderRadius="md"
    px="3"
    colorScheme={color}
  >
    {copyable && (
      <Box
        display="inline"
        cursor="pointer"
        mr="2"
        onClick={() => navigator.clipboard.writeText(name)}
      >
        <CopyIcon />
      </Box>
    )}
    {name}
  </Code>
);

export default LevelName;
