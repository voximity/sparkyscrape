import {
  Badge,
  Code,
  Table,
  TableContainer,
  Tbody,
  Td,
  Th,
  Thead,
  Tr,
} from '@chakra-ui/react';
import { useSelector } from 'react-redux';
import { selectChannelPastGames } from '../api/channels';
import Difficulty from './Difficulty';

const PastGames = ({ id }: { id: string }) => {
  const past_games = useSelector(selectChannelPastGames(id!));

  return (
    <TableContainer>
      <Table variant="simple" size="sm">
        <Thead>
          <Tr>
            <Th>Difficulty</Th>
            <Th>My guess</Th>
            <Th>Actual answer</Th>
            <Th>Result</Th>
            <Th isNumeric>Distance</Th>
          </Tr>
        </Thead>
        <Tbody>
          {past_games?.map((game) => (
            <Tr>
              <Td>
                <Difficulty difficulty={game.difficulty} />
              </Td>
              <Td>
                {game.guess ? <Code>{game.guess}</Code> : <i>No guess</i>}
              </Td>
              <Td>
                {game.result?.type === 'win' && (
                  <>
                    <Code>{game.result.answer}</Code>{' '}
                    {game.result.incorrect && (
                      <Badge colorScheme="red">Incorrect</Badge>
                    )}
                  </>
                )}
              </Td>
              <Td>
                {game.result?.type === 'timeout' ? (
                  <Badge>Timed out</Badge>
                ) : (
                  game.result?.type === 'win' && (
                    <>
                      {game.result.answer === game.guess ? (
                        <Badge colorScheme="green">Correct</Badge>
                      ) : game.result.incorrect ? (
                        <Badge colorScheme="red">Incorrect, poor match</Badge>
                      ) : (
                        <Badge colorScheme="blue">Didn't know</Badge>
                      )}
                    </>
                  )
                )}
              </Td>
              <Td isNumeric>
                {game.distance !== undefined ? (
                  <>{game.distance.toFixed(2)}</>
                ) : (
                  <i>No guess</i>
                )}
              </Td>
            </Tr>
          ))}
        </Tbody>
      </Table>
    </TableContainer>
  );
};

export default PastGames;
