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
    
from .graphql_querier import mirror_history_query
import pandas as pd
import os
import time
THRESHOLD = 0.5
# Percentage to deweight non-cross assets
X = 0.2
class BullishCrossRecomposer:
    def __init__(self, asset_addresses, use_test_data=False, minperiod=5, maxperiod=50, lookahead = 200, bar_length = 30):
        """ 
        Defaults to 30min tick and 200 bars of data (around 4 days).
        Asset names can also be addresses
        """
        self.use_test_data = use_test_data
        self.minperiod=minperiod 
        self.maxperiod=maxperiod
        self.asset_addresses = asset_addresses
        self.lookahead = lookahead

        # Use test data from Yahoo Finance
        if self.use_test_data:
            self.count = 5
            cwd = os.path.dirname(os.path.abspath(__file__))
            csv_format = cwd + '/{}.csv'
            self.closes = {a: pd.read_csv(csv_format.format(a))['Close'] for a in asset_addresses}
        else:
            self.bar_length = bar_length

    def self_opt_ma(self, data):
        """
        The sample implementation (code at the end of the article) will calculate all moving 
        averages within a given parameter range (eg. 5 bars to 200 bars), calculate the winning 
        percentage (rising bars) on the next bar, and then pick the best performing period length.
        """

        if self.use_test_data:
            close = data[self.count:self.count + self.lookahead].reset_index(drop=True)
            # yesterday
            c1 = data[self.count-1:self.count - 1 + self.lookahead].reset_index(drop=True)
            # 3 days ago
            c3 = data[self.count-3:self.count - 3 + self.lookahead].reset_index(drop=True)
        else:
            raise NotImplementedError

        counter_list = []
        win_list = []
        win_percentage_list = []

        # loop over all MA candidates
        for i in range(self.minperiod, self.maxperiod+1):
            av_c1 = c1.rolling(i).mean().reset_index(drop=True)
            av_c3 = c3.rolling(i).mean().reset_index(drop=True)
            # Total count: times av_c1 beats av_c3
            counter = (av_c1 > av_c3).sum()
            # Wins: When current close > yesterday's with av_c1 > av_c3 signal
            wins = ((av_c1 > av_c3) & (close > c1)).sum()
            counter_list.append(counter)
            win_list.append(wins)
            if counter != 0:
                win_percentage_list.append(wins / counter)
            else:
                win_percentage_list.append(0)

        smoothed_win_percentage_list = []
        # smoothing
        for j in range(2, len(win_percentage_list) - 2 - 1):
            smoothed_win_percentage = sum(win_percentage_list[j-2 : j + 2 + 1])/5
            smoothed_win_percentage_list.append(smoothed_win_percentage)

        # choose best MA candidate after smoothing
        best_period = self.minperiod + 2 + max(enumerate(smoothed_win_percentage_list), key=lambda x: x[1])[0]
        best_pwp = max(smoothed_win_percentage_list)
        av = close.rolling(best_period).mean().reset_index(drop=True)
        price_gt_ma = close.iat[-1] > av.iat[-1]
        return best_pwp, price_gt_ma

    def get_mcaps(self, asset_names):
        if asset_names != ['mFB', 'mTSLA', 'mGOOGL']:
            raise NotImplementedError
        else:
            # Mock market caps in billions
            return {
                'mFB': 3, 
                'mTSLA': 1, 
                'mGOOGL': 6
            }

    async def cross_weighting(self):
        has_cross, all_cross = False, True
        best_pwps = {}
        non_cross_assets = []
        asset_addresses = self.asset_addresses

        if not self.use_test_data:
            to = round(time.time() * 1000)

            # Need at least (self.lookahead + 3) pieces of historical data
            from_time = to - (self.lookahead + 4) * self.bar_length * 1000

            data = [await mirror_history_query(a, self.bar_length, from_time, to) for a in self.asset_addresses]
            import pdb; pdb.set_trace()
            # Might need asset names from CMC
            asset_names, max_timestamps, closes, mcs = zip(*data)
            asset_data = {name: mc for name, mc in zip(asset_names, mcs)}
            self.closes = {name: mc for name, closes in zip(asset_names, pd.Series(closes))}
            names_to_contracts = {name: addrs for name, addrs in zip(asset_names, self.asset_addresses)}
        else:
            asset_names = asset_addresses
            asset_data = self.get_mcaps(asset_names)

        # Calculate best_pwps and assets with crosses
        for asset, asset_closes in self.closes.items():
            best_pwp, price_gt_ma = self.self_opt_ma(asset_closes)
            if price_gt_ma and best_pwp > THRESHOLD:
                has_cross = True
                best_pwps[asset] = best_pwp
            else:
                all_cross = False
                non_cross_assets.append(asset)
        
        asset_mcaps = list(asset_data.values())
        print(asset_names, asset_mcaps)
        # All assets have crosses
        if all_cross:
            diffs = [best_pwps[asset] - THRESHOLD for asset in asset_names]
            denom = sum([asset_mcaps[i] * diffs[i] for i in range(len(asset_names))])
            target = {asset_names[i]: (asset_mcaps[i] * diffs[i])/denom for i in range(len(asset_names))}
        else:
            denom = sum(asset_mcaps)
            target = {asset_names[i]: asset_mcaps[i]/denom for i in range(len(asset_names))}
            print("Original allocation: {}".format(target))
            # Some asset has a cross, then divide up X share of non-cross assets to cross assets
            if has_cross:
                non_cross_pool = 0
                for asset_name in non_cross_assets:
                    share = X * target[asset_name]
                    non_cross_pool += share
                    target[asset_name] -= share
                    print("{} does not have a cross. Pool takes {}".format(asset_name, share))
                print("Total Non-Cross Pool: {}".format(non_cross_pool))
                cross_assets = list(best_pwps.keys())
                cross_diffs = [best_pwps[asset] - THRESHOLD for asset in cross_assets]
                cross_asset_mcaps = [asset_data[asset] for asset in cross_assets]
                denom = sum([cross_asset_mcaps[i] * cross_diffs[i] for i in range(len(cross_assets))])
                for i in range(len(cross_assets)):
                    cross_asset = cross_assets[i]
                    new_weight = (cross_asset_mcaps[i] * cross_diffs[i])/denom
                    target[cross_asset] += new_weight * non_cross_pool
                    print("{} target weight updated to {}".format(cross_asset, new_weight))
        assets, target_weights = zip(*target.items())
        if not self.use_test_data:
            assets = [names_to_contracts[a] for a in assets]

        return list(assets), list(target_weights)
    
    async def recompose(self):
        if self.use_test_data:
            self.count += 1
        assets, target_weights = await self.cross_weighting()
        print(assets, target_weights)
        target_weights = [int(100 * target_weight) for target_weight in target_weights]
        return assets, target_weights