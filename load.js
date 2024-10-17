import dotenv from "dotenv"
import {Connection,keypair,PublicKey} from "@solana/web3.js"

const connection=new Connection(process.env.NETWORK);
const wallet=keypair.fromSecretKey(Uint8Array.from(JSON.parse(process.env.WALLET_SECRET_KEY)));