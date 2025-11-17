// SPDX-License-Identifier: GPL-3.0-or-later

use apca::api::v2::order::Amount as ApcaAmount;
use apca::api::v2::order::Order as ApcaOrder;
use apca::api::v2::order::Status as ApcaOrderStatus;
use apca::api::v2::order::Type;
use apca::api::v2::position::Position;
use irontrade::api::common::Amount as IronTradeAmount;
use irontrade::api::response::{
    OpenPosition as IronTradeOpenPosition, Order as IronTradeOrder,
    OrderStatus as IronTradeOrderStatus, OrderType as IronTradeOrderType,
};

pub struct Amount(pub IronTradeAmount);

impl From<ApcaAmount> for Amount {
    fn from(amount: ApcaAmount) -> Self {
        match amount {
            ApcaAmount::Quantity { quantity } => Amount(IronTradeAmount::Quantity { quantity }),
            ApcaAmount::Notional { notional } => Amount(IronTradeAmount::Notional { notional }),
        }
    }
}

impl From<Amount> for ApcaAmount {
    fn from(amount: Amount) -> Self {
        match amount.0 {
            IronTradeAmount::Quantity { quantity } => Self::Quantity { quantity },
            IronTradeAmount::Notional { notional } => Self::Notional { notional },
        }
    }
}

pub struct OpenPosition(pub IronTradeOpenPosition);

impl From<Position> for OpenPosition {
    fn from(position: Position) -> Self {
        Self(IronTradeOpenPosition {
            asset_symbol: position.symbol.to_string(),
            average_entry_price: Some(position.average_entry_price),
            quantity: position.quantity,
            market_value: position.market_value,
        })
    }
}

pub struct OrderStatus(pub IronTradeOrderStatus);

impl From<ApcaOrderStatus> for OrderStatus {
    fn from(status: ApcaOrderStatus) -> Self {
        match status {
            ApcaOrderStatus::New => OrderStatus(IronTradeOrderStatus::New),
            ApcaOrderStatus::PartiallyFilled => OrderStatus(IronTradeOrderStatus::PartiallyFilled),
            ApcaOrderStatus::Filled => OrderStatus(IronTradeOrderStatus::Filled),
            ApcaOrderStatus::Expired => OrderStatus(IronTradeOrderStatus::Expired),
            _ => OrderStatus(IronTradeOrderStatus::Unimplemented),
        }
    }
}

pub struct OrderType(pub IronTradeOrderType);

impl From<Type> for OrderType {
    fn from(type_: Type) -> Self {
        match type_ {
            Type::Market => OrderType(IronTradeOrderType::Market),
            Type::Limit => OrderType(IronTradeOrderType::Limit),
            _ => todo!(),
        }
    }
}

pub struct Order(pub IronTradeOrder);

impl From<ApcaOrder> for Order {
    fn from(order: ApcaOrder) -> Self {
        let amount: Amount = order.amount.into();
        let amount = amount.0;

        let status: OrderStatus = order.status.into();
        let status = status.0;

        let type_: OrderType = order.type_.into();
        let type_ = type_.0;

        Self(IronTradeOrder {
            order_id: order.id.to_string(),
            asset_symbol: order.symbol,
            filled_quantity: order.filled_quantity,
            amount,
            average_fill_price: order.average_fill_price,
            status,
            type_,
        })
    }
}
