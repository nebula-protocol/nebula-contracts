import './App.css';
import { Provider } from 'react-redux';
import { Switch, Route, BrowserRouter } from 'react-router-dom';

import { store } from './store';

import Home from './components/Home';
import Detail from './components/Detail';
import Create from './components/Redeem';
import Redeem from './components/Create';
import Navbar from './components/Navbar';

import { Segment, Container } from 'semantic-ui-react';

import './App.css';

function App() {
  return (
    <Provider store={store}>
      <BrowserRouter>
        <Container>
          <Segment id="app-container">
            <Navbar />
            <br />
            <br />
            <Switch>
              <Route exact path="/" component={Home} />
              <Route path="/:basketName/create" component={Create} />
              <Route path="/:basketName/redeem" component={Redeem} />
              <Route path="/:basketName" component={Detail} />
            </Switch>
          </Segment>
        </Container>
      </BrowserRouter>
    </Provider>
  );
}

export default App;
