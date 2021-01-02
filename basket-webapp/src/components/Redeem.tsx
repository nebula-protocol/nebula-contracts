import { Form, Button, Input } from 'semantic-ui-react';
import { useParams } from 'react-router-dom';

export default function Redeem() {
  const { basketName } = useParams() as any;
  return (
    <div>
      <h1>Redeem - {basketName}</h1>
      <Form>
        <Form.Field
          id="form-input-control-first-name"
          control={Input}
          label="Amount to burn"
          placeholder="121.000"
        />
        <Form.Field
          id="form-button-control-public"
          control={Button}
          content="Confirm"
        />
      </Form>
    </div>
  );
}
