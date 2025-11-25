// SPDX-License-Identifier: GPL-3.0-or-later

use crate::convert::{Amount, OpenPosition, Order};
use anyhow::Result;
use apca::api::v2::asset::Symbol;
use apca::api::v2::order::{Side, TimeInForce, Type};
use apca::api::v2::orders::{ListReq, Status};
use apca::api::v2::{account, order, orders, position};
use apca::{ApiInfo, Client};
use irontrade::api::client::IronTradeClient;
use irontrade::api::common::{Order as IronTradeOrder, OrderSide};
use irontrade::api::request::OrderRequest;
use irontrade::api::response::{
    GetCashResponse, GetOpenPositionResponse, GetOrdersResponse, OrderResponse,
};

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
    async fn place_order(&mut self, req: OrderRequest) -> Result<OrderResponse> {
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

        Ok(OrderResponse { order_id })
    }

    async fn get_orders(&self) -> Result<GetOrdersResponse> {
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

        Ok(GetOrdersResponse { orders })
    }

    async fn get_cash(&self) -> Result<GetCashResponse> {
        let account = self.apca_client.issue::<account::Get>(&()).await?;
        Ok(GetCashResponse { cash: account.cash })
    }

    async fn get_open_position(&self, asset_symbol: &str) -> Result<GetOpenPositionResponse> {
        let position = self
            .apca_client
            .issue::<position::Get>(&Symbol::Sym(asset_symbol.into()))
            .await?;

        let open_position: OpenPosition = position.into();
        let open_position = open_position.0;

        Ok(GetOpenPositionResponse { open_position })
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
    async fn buy_market_returns_order_id() {
        let mut client = create_client();
        let order_id = client
            .place_order(OrderRequest {
                asset_pair: AssetPair::from_str("BTC/USD").unwrap(),
                amount: Amount::Notional {
                    notional: Num::from(20),
                },
                side: OrderSide::Buy,
                limit_price: None,
            })
            .await
            .unwrap()
            .order_id;

        assert_ne!(order_id, "")
    }

    #[tokio::test]
    async fn sell_market_returns_order_id() {
        let mut client = create_client();

        let buy_order_id = client
            .place_order(OrderRequest {
                asset_pair: AssetPair::from_str("AAVE/USD").unwrap(),
                amount: Amount::Notional {
                    notional: Num::from(20),
                },
                side: OrderSide::Buy,
                limit_price: None,
            })
            .await
            .unwrap()
            .order_id;

        loop {
            let orders = client.get_orders().await.unwrap().orders;
            let order_ids: Vec<String> =
                orders.iter().map(|order| order.order_id.clone()).collect();
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
                asset_pair: AssetPair::from_str("AAVE/USD").unwrap(),
                amount: Amount::Notional {
                    notional: Num::from(10),
                },
                side: OrderSide::Sell,
                limit_price: None,
            })
            .await
            .unwrap()
            .order_id;

        assert_ne!(order_id, "")
    }

    // TODO: Run this test atomically
    #[tokio::test]
    async fn get_orders() {
        let mut client = create_client();
        let pre_existing_orders = client.get_orders().await.unwrap().orders;

        if pre_existing_orders.is_empty() {
            client
                .place_order(OrderRequest {
                    asset_pair: AssetPair::from_str("BTC/USD").unwrap(),
                    amount: Amount::Notional {
                        notional: Num::from(20),
                    },
                    side: OrderSide::Buy,
                    limit_price: None,
                })
                .await
                .unwrap();

            let orders = client.get_orders().await.unwrap().orders;

            assert!(orders.len() > 0)
        }
    }

    #[tokio::test]
    async fn get_cash() {
        let client = create_client();
        let cash = client.get_cash().await.unwrap().cash;
        assert!(cash > Num::from(0))
    }

    #[tokio::test]
    async fn get_open_position() {
        let mut client = create_client();

        let buy_order_id = client
            .place_order(OrderRequest {
                asset_pair: AssetPair::from_str("BTC/USD").unwrap(),
                amount: Amount::Notional {
                    notional: Num::from(20),
                },
                side: OrderSide::Buy,
                limit_price: None,
            })
            .await
            .unwrap()
            .order_id;

        loop {
            let orders = client.get_orders().await.unwrap().orders;
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
            .await
            .unwrap()
            .open_position;

        assert_eq!(position.asset_symbol, "BTCUSD")
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
