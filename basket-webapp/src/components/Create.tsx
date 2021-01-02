import { useParams } from 'react-router-dom';
import { Form, Button, Input } from 'semantic-ui-react';

export default function Create() {
  const { basketName } = useParams() as any;

  return (
    <div>
      <h1>Create {basketName}</h1>
      <Form>
        <Form.Field
          id="form-input-control-first-name"
          control={Input}
          label="Amount of mAAPL"
          placeholder="121.000"
        />
        <Form.Field
          id="form-input-control-first-name"
          control={Input}
          label="Amount of mNFLX"
          placeholder="21.000"
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
