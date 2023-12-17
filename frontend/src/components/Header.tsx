import { Box, Divider, Flex, Heading, Link } from '@chakra-ui/react';
import { useSelector } from 'react-redux';
import { ReactRouterLink } from '../App';
import { selectChannels } from '../api/channels';
import Difficulty from './Difficulty';

const Header = () => {
  const channels = useSelector(selectChannels);

  return (
    <>
      <Box w="full" px="8" py="4">
        <Flex direction="row" align="center" gap="10">
          <Link as={ReactRouterLink} to="/">
            <Heading size="lg">sparkyscrape</Heading>
          </Link>
          <Link as={ReactRouterLink} to="guess" fontWeight="bold">
            Guess
          </Link>
          {Object.entries(channels).map(([id, channel]) => (
            <Link
              key={id}
              as={ReactRouterLink}
              to={`/channel/${id}`}
              fontWeight="bold"
            >
              {channel.name}
              {channel.game?.difficulty !== undefined &&
                !channel.game?.result && (
                  <>
                    {' '}
                    <Difficulty difficulty={channel.game.difficulty} />
                  </>
                )}
            </Link>
          ))}
        </Flex>
      </Box>
      <Divider />
    </>
  );
};

export default Header;
