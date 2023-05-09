import { Column, Entity, PrimaryGeneratedColumn } from "typeorm";

@Entity("tx")
export class Tx {
  @PrimaryGeneratedColumn()
  id: number;

  @Column("varchar", { length: 66, nullable: false })
  tx_eth_hash: string;

  @Column("varchar", { length: 66, nullable: true })
  tx_glitch_hash: string;

  @Column("varchar", { length: 42, nullable: false })
  from_eth_address: string;

  @Column("varchar", { length: 49, nullable: true })
  to_glitch_address: string;

  @Column("varchar", { length: 255, nullable: false })
  amount: string;

  @Column("varchar", { length: 255, nullable: true })
  business_fee_amount: string;

  @Column("varchar", { length: 255, nullable: true })
  business_fee_percentage: string;
}
