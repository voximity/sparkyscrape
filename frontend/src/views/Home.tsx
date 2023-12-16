import { Container, Flex, Heading, Spinner, Text } from '@chakra-ui/react';
import { useSelector } from 'react-redux';
import { selectConnected } from '../api/store';

const Home = () => {
  const connected = useSelector(selectConnected);

  if (!connected)
    return (
      <Container maxW="container.lg">
        <Flex direction="row" align="center">
          <Spinner />
          <Text>Establishing connection...</Text>
        </Flex>
      </Container>
    );

  return (
    <Container maxW="container.lg">
      <Heading>Hello, world</Heading>
    </Container>
  );
};

export default Home;
