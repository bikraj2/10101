use crate::channel::Channel;
use crate::node::LiquidityRequest;
use crate::node::Node;
use crate::node::Storage;
use crate::MillisatAmount;
use crate::PaymentFlow;
use crate::PaymentInfo;
use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use autometrics::autometrics;
use bitcoin::hashes::hex::ToHex;
use bitcoin::hashes::sha256;
use bitcoin::hashes::Hash;
use bitcoin::secp256k1::PublicKey;
use bitcoin::secp256k1::Secp256k1;
use bitcoin::Network;
use lightning::ln::channelmanager::Retry;
use lightning::ln::channelmanager::RetryableSendFailure;
use lightning::ln::channelmanager::MIN_CLTV_EXPIRY_DELTA;
use lightning::ln::PaymentHash;
use lightning::routing::gossip::RoutingFees;
use lightning::routing::router::RouteHint;
use lightning::routing::router::RouteHintHop;
use lightning_invoice::payment::pay_invoice;
use lightning_invoice::payment::pay_zero_value_invoice;
use lightning_invoice::payment::PaymentError;
use lightning_invoice::Currency;
use lightning_invoice::Invoice;
use lightning_invoice::InvoiceBuilder;
use lightning_invoice::InvoiceDescription;
use std::fmt;
use std::fmt::Formatter;
use std::time::Duration;
use std::time::SystemTime;
use time::OffsetDateTime;

impl<P> Node<P>
where
    P: Storage,
{
    pub fn create_invoice(
        &self,
        amount_in_sats: u64,
        description: String,
        expiry: u32,
    ) -> Result<Invoice> {
        lightning_invoice::utils::create_invoice_from_channelmanager(
            &self.channel_manager,
            self.keys_manager.clone(),
            self.logger.clone(),
            self.get_currency(),
            Some(amount_in_sats * 1000),
            description,
            expiry,
            None,
        )
        .map_err(|e| anyhow!(e))
    }

    /// Creates an invoice which is meant to be intercepted.
    ///
    /// The `final_route_hint_hop` argument is generated by the intercepting node, so that it can
    /// identify the corresponding HTLC once it is routed through it. This can be used to open
    /// just-in-time channels.
    pub fn create_invoice_with_route_hint(
        &self,
        amount_in_sats: Option<u64>,
        invoice_expiry: Option<Duration>,
        description: String,
        final_route_hint_hop: RouteHintHop,
    ) -> Result<Invoice> {
        let invoice_expiry = invoice_expiry
            .unwrap_or_else(|| Duration::from_secs(lightning_invoice::DEFAULT_EXPIRY_TIME));
        let amount_msat = amount_in_sats.map(|x| x * 1000);
        let (payment_hash, payment_secret) = self
            .channel_manager
            .create_inbound_payment(amount_msat, invoice_expiry.as_secs() as u32, None)
            .map_err(|_| anyhow!("Failed to create inbound payment"))?;
        let invoice_builder = InvoiceBuilder::new(self.get_currency())
            .payee_pub_key(self.info.pubkey)
            .description(description)
            .expiry_time(invoice_expiry)
            .payment_hash(sha256::Hash::from_slice(&payment_hash.0)?)
            .payment_secret(payment_secret)
            .timestamp(SystemTime::now())
            // lnd defaults the min final cltv to 9 (according to BOLT 11 - the recommendation has
            // changed to 18) 9 is not safe to use for ldk, because ldk mandates that
            // the `cltv_expiry_delta` has to be greater than `HTLC_FAIL_BACK_BUFFER`
            // (23).
            .min_final_cltv_expiry_delta(MIN_CLTV_EXPIRY_DELTA.into())
            .private_route(RouteHint(vec![final_route_hint_hop]));

        let invoice_builder = match amount_msat {
            Some(msats) => invoice_builder.amount_milli_satoshis(msats),
            None => invoice_builder,
        };

        let node_secret = self.keys_manager.get_node_secret_key();

        let signed_invoice = invoice_builder
            .build_raw()?
            .sign::<_, ()>(|hash| {
                let secp_ctx = Secp256k1::new();
                Ok(secp_ctx.sign_ecdsa_recoverable(hash, &node_secret))
            })
            .map_err(|_| anyhow!("Failed to sign invoice"))?;
        let invoice = Invoice::from_signed(signed_invoice)?;

        Ok(invoice)
    }

    fn get_currency(&self) -> Currency {
        match self.network {
            Network::Bitcoin => Currency::Bitcoin,
            Network::Testnet => Currency::BitcoinTestnet,
            Network::Regtest => Currency::Regtest,
            Network::Signet => Currency::Signet,
        }
    }

    /// First step to open a just-in-time (JIT) channel.
    ///
    /// We register the target node's ID with a newly generated intercept SCID, which will be used
    /// to identify the incoming HTLC to be used to open the JIT channel.
    ///
    /// # Returns
    ///
    /// A [`RouteHintHop`] indicating that we are the node that will open the JIT channel. We insert
    /// the `intercept_scid` in the `short_channel_id` field so that our node is able to identify
    /// the corresponding incoming HTLC once the payment is sent.
    ///
    /// We also specify the [`RoutingFees`] to ensure that the payment is made in accordance with
    /// the fees that we want to charge.
    ///
    /// # Errors
    ///
    /// An error if the user already has a channel. Use `prepare_payment_with_route_hint` instead.
    pub fn prepare_onboarding_payment(
        &self,
        liquidity_request: LiquidityRequest,
    ) -> Result<RouteHintHop> {
        let trader_id = liquidity_request.trader_id;
        let user_channel_id = liquidity_request.user_channel_id;

        let intercept_scid = self.channel_manager.get_intercept_scid();
        self.fake_channel_payments
            .lock()
            .insert(intercept_scid, liquidity_request.clone());

        let ldk_config = self.ldk_config.read();

        let route_hint_hop = RouteHintHop {
            src_node_id: self.info.pubkey,
            short_channel_id: intercept_scid,
            fees: RoutingFees {
                base_msat: ldk_config.channel_config.forwarding_fee_base_msat,
                proportional_millionths: ldk_config
                    .channel_config
                    .forwarding_fee_proportional_millionths,
            },
            cltv_expiry_delta: MIN_CLTV_EXPIRY_DELTA,
            htlc_minimum_msat: None,
            htlc_maximum_msat: None,
        };

        let channel = Channel::new_jit_channel(
            user_channel_id,
            trader_id,
            liquidity_request.liquidity_option_id,
        );
        self.storage.upsert_channel(channel).with_context(|| {
            format!(
                "Failed to insert shadow JIT channel for counterparty {trader_id} \
                             with user channel id {user_channel_id}"
            )
        })?;

        tracing::info!(
            %user_channel_id,
            %trader_id,
            interceptable_route_hint_hop = ?route_hint_hop,
            "Registered interest to open JIT channel"
        );
        Ok(route_hint_hop)
    }

    pub fn prepare_payment_with_route_hint(&self, target_node: PublicKey) -> Result<RouteHintHop> {
        let channels = self.channel_manager.list_channels();
        let channel = channels
            .iter()
            .find(|channel| channel.counterparty.node_id == target_node)
            .with_context(|| format!("Couldn't find channel for {target_node}"))?;

        let short_channel_id = channel.short_channel_id.with_context(|| {
            format!(
                "Couldn't find short channel id for channel: {}, trader_id={target_node}",
                channel.channel_id.to_hex()
            )
        })?;

        let ldk_config = self.ldk_config.read();

        let route_hint_hop = RouteHintHop {
            src_node_id: self.info.pubkey,
            short_channel_id,
            fees: RoutingFees {
                base_msat: ldk_config.channel_config.forwarding_fee_base_msat,
                proportional_millionths: ldk_config
                    .channel_config
                    .forwarding_fee_proportional_millionths,
            },
            cltv_expiry_delta: MIN_CLTV_EXPIRY_DELTA,
            htlc_minimum_msat: None,
            htlc_maximum_msat: None,
        };

        tracing::info!(
            peer_id = %target_node,
            route_hint_hop = ?route_hint_hop,
            "Created route hint for payment to private channel"
        );

        Ok(route_hint_hop)
    }

    pub fn pay_invoice(&self, invoice: &Invoice, amount: Option<u64>) -> Result<()> {
        let (result, amt_msat) = match invoice.amount_milli_satoshis() {
            Some(_) => {
                let result = pay_invoice(invoice, Retry::Attempts(10), &self.channel_manager);
                (result, invoice.amount_milli_satoshis().expect("to be set"))
            }
            None => {
                let amount_msats =
                    amount.context("Can't pay zero amount invoice without amount")? * 1000;
                let result = pay_zero_value_invoice(
                    invoice,
                    amount_msats,
                    Retry::Attempts(10),
                    &self.channel_manager,
                );
                (result, amount_msats)
            }
        };

        let (status, err) = match result {
            Ok(payment_id) => {
                let payee_pubkey = match invoice.payee_pub_key() {
                    Some(pubkey) => *pubkey,
                    None => invoice.recover_payee_pub_key(),
                };

                tracing::info!(
                    peer_id = %payee_pubkey,
                    amount_msat = %amt_msat,
                    payment_id = %hex::encode(payment_id.0),
                    "Initiated payment"
                );

                (HTLCStatus::Pending, None)
            }
            Err(PaymentError::Invoice(err)) => {
                tracing::error!(%err, "Invalid invoice");
                anyhow::bail!(err);
            }
            Err(PaymentError::Sending(err)) => {
                tracing::error!(?err, "Failed to send payment");
                let failure_reason = retryable_send_failure_to_string(err);

                (HTLCStatus::Failed, Some(failure_reason))
            }
        };

        let description = match invoice.description() {
            InvoiceDescription::Direct(des) => des.clone().into_inner(),
            InvoiceDescription::Hash(lightning_invoice::Sha256(des)) => des.to_string(),
        };

        self.storage.insert_payment(
            PaymentHash(invoice.payment_hash().into_inner()),
            PaymentInfo {
                preimage: None,
                secret: None,
                status,
                amt_msat: MillisatAmount(Some(amt_msat)),
                fee_msat: MillisatAmount(None),
                flow: PaymentFlow::Outbound,
                timestamp: OffsetDateTime::now_utc(),
                description,
                invoice: Some(format!("{invoice}")),
            },
        )?;

        if let Some(failure_reason) = err {
            anyhow::bail!("Failed to send payment: {}, {}", failure_reason, invoice);
        }

        Ok(())
    }

    #[cfg(test)]
    pub async fn wait_for_payment_claimed(
        &self,
        hash: &sha256::Hash,
    ) -> Result<(), tokio::time::error::Elapsed> {
        self.wait_for_payment(HTLCStatus::Succeeded, hash, None)
            .await
    }

    #[autometrics]
    pub async fn wait_for_payment(
        &self,
        expected_status: HTLCStatus,
        hash: &sha256::Hash,
        timeout: Option<Duration>,
    ) -> Result<(), tokio::time::error::Elapsed> {
        assert_ne!(
            expected_status,
            HTLCStatus::Pending,
            "Waiting for pending is not a valid status"
        );
        let payment_hash = PaymentHash(hash.into_inner());

        tokio::time::timeout(timeout.unwrap_or(Duration::from_secs(10)), async {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;

                match self.storage.get_payment(&payment_hash) {
                    Ok(Some((_, PaymentInfo { status, .. }))) => {
                        tracing::debug!(
                            payment_hash = %hex::encode(hash),
                            ?status,
                            "Checking if payment has been claimed"
                        );
                        if expected_status == status {
                            return;
                        }
                    }
                    Ok(None) => {
                        tracing::debug!(
                            payment_hash = %hex::encode(hash),
                            status = "unknown",
                            "Checking if payment has been claimed"
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            payment_hash = %hex::encode(hash),
                            status = "error",
                            "Can't access store to load payment: {e:#}"
                        );
                    }
                }
            }
        })
        .await
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HTLCStatus {
    Pending,
    Succeeded,
    Failed,
}

impl fmt::Display for HTLCStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            HTLCStatus::Pending => "Pending".fmt(f),
            HTLCStatus::Succeeded => "Succeeded".fmt(f),
            HTLCStatus::Failed => "Failed".fmt(f),
        }
    }
}

fn retryable_send_failure_to_string(failure: RetryableSendFailure) -> &'static str {
    match failure {
        RetryableSendFailure::DuplicatePayment => "Duplicate payment",
        RetryableSendFailure::PaymentExpired => "Payment expired",
        RetryableSendFailure::RouteNotFound => "Route not found",
    }
}
