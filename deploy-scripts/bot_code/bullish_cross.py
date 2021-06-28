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
    
from .coinmarketcap import get_prices
import pandas as pd
THRESHOLD = 0.5
# Percentage to deweight non-cross assets
X = 0.2
class BullishCrossRecomposer:
    def __init__(self, asset_names, use_test_data=False, minperiod=5, maxperiod=50, lookahead = 200):
        self.use_test_data = use_test_data
        self.minperiod=minperiod 
        self.maxperiod=maxperiod
        self.asset_names = asset_names
        self.lookahead = lookahead

        # Use test data from Yahoo Finance
        if self.use_test_data:
            self.count = 5
            self.closes = {a: pd.read_csv('/Users/Manav/Documents/crypto_projects/nebula/deploy-scripts/bot_code/' + a + '.csv')['Close'] for a in asset_names}
        else:
            raise NotImplementedError # Should make data follow correct format in this case

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
        asset_names = self.asset_names
        # Calculate best_pwps and assets with crosses
        for asset, asset_closes in self.closes.items():
            best_pwp, price_gt_ma = self.self_opt_ma(asset_closes)
            if price_gt_ma and best_pwp > THRESHOLD:
                has_cross = True
                best_pwps[asset] = best_pwp
            else:
                all_cross = False
                non_cross_assets.append(asset)
        asset_data = self.get_mcaps(asset_names)
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
            # Some asset has a cross, then divide up X share of non-cross assets to cross assets
            if has_cross:
                non_cross_pool = 0
                for asset_name in non_cross_assets:
                    share = X * target[asset_name]
                    non_cross_pool += share
                    target[asset_name] -= share
                cross_assets = list(best_pwps.keys())
                cross_diffs = [best_pwps[asset] - THRESHOLD for asset in cross_assets]
                cross_asset_mcaps = [asset_data[asset] for asset in cross_assets]
                denom = sum([cross_asset_mcaps[i] * cross_diffs[i] for i in range(len(cross_assets))])
                for i in range(len(cross_assets)):
                    cross_asset = cross_assets[i]
                    target[cross_asset] = (cross_asset_mcaps[i] * cross_diffs[i])/denom
            assets, target_weight = zip(*target.items())
            return list(assets), list(target_weight)
    
    # background contracts needed to create cluster contracts
    async def recompose(self):
        self.count += 1
        self_optimized = []
        assets, target_weight = await self.cross_weighting()
        print(assets, target_weight)
        return assets, target_weight