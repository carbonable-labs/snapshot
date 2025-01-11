import { hash } from "starknet";
import { keys, map } from "lodash";
import { type, version } from "os";

import { appendFile } from "node:fs";

export const config = {
  streamUrl: "https://mainnet.starknet.a5a.ch",
  startingBlock: 456282,
  network: "starknet",
  finality: "DATA_STATUS_ACCEPTED",
  filter: {
    events: [
      {
        fromAddress:
          "0x03d25473be5a6316f351e8f964d0c303357c006f7107779f648d9879b7c6d58a",
        keys: [
          "0x9149d2123147c5f43d258257fef0b7b969db78269369ebcf5ebb9eef8592f2",
        ],
      },
      {
        fromAddress:
          "0x00426d4e86913759bcc49b7f992b1fe62e6571e8f8089c23d95fea815dbad471",
        keys: [
          "0x9149d2123147c5f43d258257fef0b7b969db78269369ebcf5ebb9eef8592f2",
        ],
      },
      {
        fromAddress:
          "0x03afe61732ed9b226309775ac4705129319729d3bee81da5632146ffd72652ae",
        keys: [
          "0x9149d2123147c5f43d258257fef0b7b969db78269369ebcf5ebb9eef8592f2",
        ],
      },
    ],
  },
  sinkType: "console",
};
let fct_map = {
  "0x034e55c1cd55f1338241b50d352f0e91c7e4ffad0e4271d64eb347589ebdfd16": "mint",
  "0x0099cd8bde557814842a3121e8ddfd433a539b8c9f14bf31ebf108d12e6196e9":
    "transfer",
};
// This transform does nothing.
export default async function transform(block: BlockData) {
  const ret = block.events.forEach((x) => {
    const depositer = x.event.data[0];
    const yielder = x.event.fromAddress;

    appendFile(`yielder_depositers/${yielder}.txt`, depositer + "\n", (err) => {
      if (err) throw err;
      console.log('The "data to append" was appended to file!');
    });
  });

  appendFile("message.txt", JSON.stringify(ret) + "\n", (err) => {
    if (err) throw err;
    console.log('The "data to append" was appended to file!');
  });

  return ret;
} // Define the top-level interface for the entire array
interface BlockData {
  status: string;
  events: Event[];
}

// Define the interface for each event in the events array
interface Event {
  transaction: Transaction;
  event: EventDetails;
}

// Define the interface for the transaction object
interface Transaction {
  meta: TransactionMeta;
  invokeV3: InvokeV3;
}

// Define the interface for the transaction metadata
interface TransactionMeta {
  hash: string;
  signature: string[];
  nonce: string;
  version: string;
  resourceBounds: ResourceBounds;
  nonceDataAvailabilityMode: string;
  feeDataAvailabilityMode: string;
  transactionIndex: string;
}

// Define the interface for resource bounds within the metadata
interface ResourceBounds {
  l1Gas: Gas;
  l2Gas: Gas;
}

// Define the interface for gas details
interface Gas {
  maxAmount?: string; // Optional because l2Gas.maxPricePerUnit is empty
  maxPricePerUnit?: PricePerUnit;
}

// Define the interface for price per unit
interface PricePerUnit {
  high: string;
}

// Define the interface for invokeV3
interface InvokeV3 {
  senderAddress: string;
  calldata: string[];
}

// Define the interface for event details
interface EventDetails {
  fromAddress: string;
  keys: string[];
  data: string[];
  index: string;
}
