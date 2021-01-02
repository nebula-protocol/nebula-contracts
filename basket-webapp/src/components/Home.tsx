import { Link } from 'react-router-dom';
import { Header, List, Button, Image } from 'semantic-ui-react';

export default function Home() {
  return (
    <div>
      <Header as="h1">Available Baskets</Header>
      <List divided verticalAlign="middle">
        <List.Item>
          <List.Content floated="right">100.20 UST</List.Content>
          <Image avatar src="/images/avatar/small/lena.png" />
          <List.Content>
            <List.Header>
              <Link to="/B-DFIP">
                <code>B-DFIP</code>
              </Link>
            </List.Header>
            <List.Description>DeFi Pulse Index</List.Description>
          </List.Content>
        </List.Item>
        <List.Item>
          <List.Content floated="right">2392.22 UST</List.Content>
          <Image avatar src="/images/avatar/small/lindsay.png" />
          <List.Content>
            <List.Header as="a">
              <Link to="/B-USTECH">
                <code>B-USTECH</code>
              </Link>
            </List.Header>
            <List.Description>Top US Tech Stocks</List.Description>
          </List.Content>
        </List.Item>
        <List.Item>
          <List.Content floated="right">1223.21 UST</List.Content>
          <Image avatar src="/images/avatar/small/mark.png" />
          <List.Content>
            <List.Header as="a">
              <Link to="/B-ALT">
                <code>B-ALT</code>
              </Link>
            </List.Header>
            <List.Description>Top Altcoins Index</List.Description>
          </List.Content>
        </List.Item>
      </List>
    </div>
  );
}
