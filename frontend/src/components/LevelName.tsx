import { CheckIcon, CopyIcon } from '@chakra-ui/icons';
import { Box, Code, Tooltip } from '@chakra-ui/react';
import { ComponentProps, useRef, useState } from 'react';

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
}) => {
  const [copied, setCopied] = useState(false);
  const timeout = useRef<NodeJS.Timeout>();
  const copy = () => {
    navigator.clipboard.writeText(name);
    setCopied(true);

    if (timeout.current) clearTimeout(timeout.current);
    timeout.current = setTimeout(() => {
      setCopied(false);
    }, 2000);
  };

  return (
    <Code
      fontSize={size ?? '2xl'}
      fontWeight="bold"
      borderRadius="md"
      px="3"
      colorScheme={color}
    >
      {copyable && (
        <Tooltip label="Copied to clipboard" isOpen={copied} hasArrow>
          <Box display="inline" cursor="pointer" mr="3" onClick={copy}>
            {copied ? <CheckIcon /> : <CopyIcon />}
          </Box>
        </Tooltip>
      )}
      {name}
    </Code>
  );
};

export default LevelName;
