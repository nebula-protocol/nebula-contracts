"""
c1=close[1];// yestedays close
c3=close[3];
for i=minperiod to maxperiod begin // loop all averages and sum up results
	av=average(c1,i);
	if av>average(c3,i) then begin
		counter[i]=counter[i]+1;
		if close>c1 then periodwin[i]=periodwin[i]+1;
	end;
end;

for i=minperiod to maxperiod begin // calc percent winning bars
	if counter[i]>0 then periodwinpercent[i]=100*periodwin[i]/counter[i];
end;
for i=minperiod+2 to maxperiod-2 begin // a little bit of smoothing
	pwpsmooth[i]=(periodwinpercent[i-2]+periodwinpercent[i-1]+periodwinpercent[i]+periodwinpercent[i+1]+periodwinpercent[i+2])/5;
end;

period=indexofhighestarray(pwpsmooth); // best period

av=average(close,period);
indication=highestarray(pwpsmooth); // winning percentage (smoothed)
"""
    
import pandas as pd

class BullishCrossRecomposer:
    def __init__(self, asset_names, use_test_data=False, minperiod=5, maxperiod=200):
        self.use_test_data = use_test_data
        self.minperiod=minperiod, 
        self.maxperiod=maxperiod,
        self.asset_names = asset_names

        # Use test data from Yahoo Finance
        if self.use_test_data:
            self.count = 5
            self.closes = {a: list(pd.read_csv(a + '.csv')['Close']) for a in asset_names}

    def self_opt_ma(data):
        """
        The sample implementation (code at the end of the article) will calculate all moving 
        averages within a given parameter range (eg. 5 bars to 200 bars), calculate the winning 
        percentage (rising bars) on the next bar, and then pick the best performing period length.
        """

        close = self.closes[self.count:self.count + self.maxperiod]
        c1 = self.closes[self.count-1:self.count - 1 + self.maxperiod]
        c3 = self.closes[self.count-3:self.count - 3 + self.maxperiod]



    # background contracts needed to create cluster contracts
    async def recompose(self, asset_names):
        self.count += 1

        self_optimized = [self_opt_ma(data) for data in self.closes]
        self_optimized = zip(self.asset_names, self_optimized)

        # pull MC data here to weight too (?)

        return ['mFB', 'mTSLA', 'mGOOGL'], [20, 20, 60]