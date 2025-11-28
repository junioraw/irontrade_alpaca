// SPDX-License-Identifier: GPL-3.0-or-later

use crate::convert::{Amount, OpenPosition, Order};
use anyhow::Result;
use apca::api::v2::asset::Symbol;
use apca::api::v2::order::{Side, TimeInForce, Type};
use apca::api::v2::orders::{ListReq, Status};
use apca::api::v2::{account, order, orders, position};
use apca::{ApiInfo, Client};
use irontrade::api::client::IronTradeClient;
use irontrade::api::common::{OpenPosition as IronTradeOpenPosition, Order as IronTradeOrder, OrderSide};
use irontrade::api::request::OrderRequest;
use num_decimal::Num;

pub struct AlpacaClient {
    apca_client: Client,
}

impl AlpacaClient {
    fn new(api_info: ApiInfo) -> Self {
        Self {
            apca_client: Client::new(api_info),
        }
    }
}

impl IronTradeClient for AlpacaClient {
    async fn place_order(&mut self, req: OrderRequest) -> Result<String> {
        let side: Side = match req.side {
            OrderSide::Buy => Side::Buy,
            OrderSide::Sell => Side::Sell,
        };

        let type_: Type;

        if req.limit_price.is_some() {
            type_ = Type::Limit;
        } else {
            type_ = Type::Market;
        }

        let amount = Amount(req.amount);
        let request = order::CreateReqInit {
            type_,
            time_in_force: TimeInForce::UntilCanceled,
            limit_price: req.limit_price,
            ..Default::default()
        }
        .init(req.asset_pair.to_string(), side, amount.into());

        let order_id = self
            .apca_client
            .issue::<order::Create>(&request)
            .await?
            .id
            .to_string();

        Ok(order_id)
    }

    async fn get_orders(&self) -> Result<Vec<IronTradeOrder>> {
        let orders: Vec<IronTradeOrder> = self
            .apca_client
            .issue::<orders::List>(&ListReq {
                status: Status::All,
                ..Default::default()
            })
            .await?
            .iter()
            .map(|order| {
                let order: Order = order.clone().into();
                order.0
            })
            .collect();

        Ok(orders)
    }

    async fn get_buying_power(&self) -> Result<Num> {
        let buying_power = self.apca_client.issue::<account::Get>(&()).await?.buying_power;
        Ok(buying_power)
    }

    async fn get_cash(&self) -> Result<Num> {
        let cash = self.apca_client.issue::<account::Get>(&()).await?.cash;
        Ok(cash)
    }

    async fn get_open_position(&self, asset_symbol: &str) -> Result<IronTradeOpenPosition> {
        let position = self
            .apca_client
            .issue::<position::Get>(&Symbol::Sym(asset_symbol.into()))
            .await?;

        let open_position: OpenPosition = position.into();
        let open_position = open_position.0;

        Ok(open_position)
    }
}

// Tests use environment variable keys for api secret, so make sure those are set to a paper test account
#[cfg(test)]
mod tests {
    use super::*;
    use apca::ApiInfo;
    use irontrade::api::common::{Amount, AssetPair, OrderStatus};
    use num_decimal::Num;
    use std::str::FromStr;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn buy_market_returns_order_id() -> Result<()> {
        let mut client = create_client();
        let order_id = client
            .place_order(OrderRequest {
                asset_pair: AssetPair::from_str("BTC/USD")?,
                amount: Amount::Notional {
                    notional: Num::from(20),
                },
                side: OrderSide::Buy,
                limit_price: None,
            })
            .await?;

        assert_ne!(order_id, "");

        Ok(())
    }

    #[tokio::test]
    async fn sell_market_returns_order_id() -> Result<()> {
        let mut client = create_client();

        let buy_order_id = client
            .place_order(OrderRequest {
                asset_pair: AssetPair::from_str("AAVE/USD")?,
                amount: Amount::Notional {
                    notional: Num::from(20),
                },
                side: OrderSide::Buy,
                limit_price: None,
            })
            .await?;

        loop {
            let orders = client.get_orders().await?;
            let buy_order = orders
                .iter()
                .find(|order| order.order_id == buy_order_id)
                .unwrap();
            if matches!(buy_order.status, OrderStatus::Filled) {
                break;
            }
            sleep(Duration::from_secs(1)).await;
        }

        let order_id = client
            .place_order(OrderRequest {
                asset_pair: AssetPair::from_str("AAVE/USD")?,
                amount: Amount::Notional {
                    notional: Num::from(10),
                },
                side: OrderSide::Sell,
                limit_price: None,
            })
            .await?;

        assert_ne!(order_id, "");

        Ok(())
    }

    // TODO: Run this test atomically
    #[tokio::test]
    async fn get_orders() -> Result<()> {
        let mut client = create_client();
        let pre_existing_orders = client.get_orders().await?;

        if pre_existing_orders.is_empty() {
            client
                .place_order(OrderRequest {
                    asset_pair: AssetPair::from_str("BTC/USD")?,
                    amount: Amount::Notional {
                        notional: Num::from(20),
                    },
                    side: OrderSide::Buy,
                    limit_price: None,
                }).await?;

            let orders = client.get_orders().await?;

            assert!(orders.len() > 0);
        }

        Ok(())
    }

    #[tokio::test]
    async fn get_cash() -> Result<()> {
        let client = create_client();
        let cash = client.get_cash().await?;
        assert!(cash > Num::from(0));
        Ok(())
    }

    #[tokio::test]
    async fn get_buying_power() -> Result<()> {
        let client = create_client();
        let buying_power = client.get_buying_power().await?;
        assert!(buying_power > Num::from(0));
        Ok(())
    }

    #[tokio::test]
    async fn get_open_position() -> Result<()> {
        let mut client = create_client();

        let buy_order_id = client
            .place_order(OrderRequest {
                asset_pair: AssetPair::from_str("BTC/USD")?,
                amount: Amount::Notional {
                    notional: Num::from(20),
                },
                side: OrderSide::Buy,
                limit_price: None,
            })
            .await?;

        loop {
            let orders = client.get_orders().await?;
            let buy_order = orders
                .iter()
                .find(|order| order.order_id == buy_order_id)
                .unwrap();
            if matches!(buy_order.status, OrderStatus::Filled) {
                break;
            }
            sleep(Duration::from_secs(1)).await;
        }

        let position = client
            .get_open_position("BTC/USD".into())
            .await?;

        assert_eq!(position.asset_symbol, "BTCUSD");

        Ok(())
    }

    fn create_client() -> AlpacaClient {
        let api_info = ApiInfo::from_env().unwrap();
        assert!(
            api_info.api_base_url.to_string().contains("paper"),
            "Use a paper account for unit testing"
        );
        AlpacaClient::new(api_info)
    }
}
