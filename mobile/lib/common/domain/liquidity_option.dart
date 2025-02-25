import 'package:get_10101/bridge_generated/bridge_definitions.dart' as bridge;
import 'package:get_10101/common/domain/model.dart';

/// A representation of a proportional fee of an amount with an optional min amount.
class ProportionalFee {
  double percentage;
  int minSats;

  ProportionalFee({required this.percentage, this.minSats = 0});

  Amount getFee(Amount amount) {
    final fee = (amount.sats / 100) * percentage;
    return fee < minSats ? Amount(minSats) : Amount(fee.ceil());
  }
}

class LiquidityOption {
  final int liquidityOptionId;
  final String title;
  final Amount tradeUpTo;
  final Amount minDeposit;
  final Amount maxDeposit;
  final ProportionalFee fee;
  final double coordinatorLeverage;

  LiquidityOption(
      {required this.liquidityOptionId,
      required this.title,
      required this.tradeUpTo,
      required this.minDeposit,
      required this.maxDeposit,
      required this.fee,
      required this.coordinatorLeverage});

  static LiquidityOption from(bridge.LiquidityOption option) {
    return LiquidityOption(
      liquidityOptionId: option.id,
      title: option.title,
      tradeUpTo: Amount(option.tradeUpToSats),
      minDeposit: Amount(option.minDepositSats),
      maxDeposit: Amount(option.maxDepositSats),
      fee: ProportionalFee(percentage: option.feePercentage, minSats: option.minFeeSats),
      coordinatorLeverage: option.coordinatorLeverage,
    );
  }
}
