import { useParams, Link } from 'react-router-dom';
import { Button } from 'semantic-ui-react';

const baskets: any = {
  'B-DFIP': {
    description: 'DeFi Pulse Index',
  },
  'B-USTECH': {
    description: 'Top US Tech Stocks',
  },
  'B-ALT': {
    description: 'Altcoin Index',
  },
};

export default function Detail() {
  const { basketName } = useParams() as any;
  const { description } = baskets[basketName];
  return (
    <div>
      <h1> {basketName} </h1>
      {description}
      <br />
      <br />
      <Button.Group>
        <Link to={`${basketName}/create`}>
          <Button>Create</Button>
        </Link>
        <Button.Or />
        <Link to={`${basketName}/redeem`}>
          <Button positive>Redeem</Button>
        </Link>
      </Button.Group>
    </div>
  );
}
