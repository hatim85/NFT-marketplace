import { actions,programs } from "@metaplex/js";
import {Metadata} from programs.metadata;

async function mintNFT(){
    const mint=await actions.createTokenMint({
        connection,
        wallet,
        uri:'',
        symbol:'NFTsymbol',
        sellerFeeBasisPoints:500,   //5% royalty fee
        creators:[{address:wallet.publicKey.toString(),share:100}]
    });
    console.log('NFT Minted: ',mint.mint.toString());
}

mintNFT.catch(console.error)