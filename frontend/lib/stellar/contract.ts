import {
  Contract,
  SorobanRpc,
  TransactionBuilder,
  Networks,
  BASE_FEE,
  Address,
  nativeToScVal,
  scValToNative,
} from "@stellar/stellar-sdk";
import { signTransaction } from "./client";
import { NETWORK_PASSPHRASE, RPC_URL, CONTRACT_ID, getNetwork } from "./client";

const server = new SorobanRpc.Server(RPC_URL);

interface ContractInvocationParams {
  method: string;
  args: any[];
  callerAddress: string;
}

async function buildAndSimulateTransaction(
  params: ContractInvocationParams
): Promise<SorobanRpc.SimulateTransactionResponse> {
  const account = await server.getAccount(params.callerAddress);
  const contract = new Contract(CONTRACT_ID);

  const tx = new TransactionBuilder(account, {
    fee: BASE_FEE,
    networkPassphrase: NETWORK_PASSPHRASE,
  })
    .addOperation(
      contract.call(params.method, ...params.args.map((arg) => nativeToScVal(arg)))
    )
    .setTimeout(30)
    .build();

  return server.simulateTransaction(tx);
}

async function buildSignAndSubmitTransaction(
  params: ContractInvocationParams
): Promise<string> {
  const account = await server.getAccount(params.callerAddress);
  const contract = new Contract(CONTRACT_ID);

  let tx = new TransactionBuilder(account, {
    fee: BASE_FEE,
    networkPassphrase: NETWORK_PASSPHRASE,
  })
    .addOperation(
      contract.call(params.method, ...params.args.map((arg) => nativeToScVal(arg)))
    )
    .setTimeout(30)
    .build();

  // Simulate to get auth and resource fees
  const simulated = await server.simulateTransaction(tx);

  if (SorobanRpc.isSimulationSuccess(simulated)) {
    tx = SorobanRpc.assembleTransaction(tx, simulated).build();
  } else {
    throw new Error(`Simulation failed: ${simulated.error}`);
  }

  // Sign with Freighter
  const signed = await signTransaction(tx.toXDR(), NETWORK_PASSPHRASE);
  const signedTx = TransactionBuilder.fromXDR(signed, NETWORK_PASSPHRASE);

  // Submit
  const result = await server.sendTransaction(signedTx);
  return result.hash;
}

export const contractClient = {
  async registerProduct(
    productId: string,
    name: string,
    origin: string,
    owner: string,
    callerAddress: string
  ): Promise<string> {
    return buildSignAndSubmitTransaction({
      method: "register_product",
      args: [productId, name, origin, new Address(owner)],
      callerAddress,
    });
  },

  async addTrackingEvent(
    productId: string,
    location: string,
    eventType: string,
    metadata: string,
    callerAddress: string
  ): Promise<string> {
    return buildSignAndSubmitTransaction({
      method: "add_tracking_event",
      args: [productId, location, eventType, metadata],
      callerAddress,
    });
  },

  async getProduct(productId: string, callerAddress: string): Promise<any> {
    const simulated = await buildAndSimulateTransaction({
      method: "get_product",
      args: [productId],
      callerAddress,
    });

    if (SorobanRpc.isSimulationSuccess(simulated)) {
      return scValToNative(simulated.results?.[0]);
    }
    throw new Error("Failed to get product");
  },

  async getTrackingEvents(productId: string, callerAddress: string): Promise<any[]> {
    const simulated = await buildAndSimulateTransaction({
      method: "get_tracking_events",
      args: [productId],
      callerAddress,
    });

    if (SorobanRpc.isSimulationSuccess(simulated)) {
      return scValToNative(simulated.results?.[0]) || [];
    }
    throw new Error("Failed to get tracking events");
  },

  async transferOwnership(
    productId: string,
    newOwner: string,
    callerAddress: string
  ): Promise<string> {
    return buildSignAndSubmitTransaction({
      method: "transfer_ownership",
      args: [productId, new Address(newOwner)],
      callerAddress,
    });
  },

  async addAuthorizedActor(
    productId: string,
    actor: string,
    callerAddress: string
  ): Promise<string> {
    return buildSignAndSubmitTransaction({
      method: "add_authorized_actor",
      args: [productId, new Address(actor)],
      callerAddress,
    });
  },

  async removeAuthorizedActor(
    productId: string,
    actor: string,
    callerAddress: string
  ): Promise<string> {
    return buildSignAndSubmitTransaction({
      method: "remove_authorized_actor",
      args: [productId, new Address(actor)],
      callerAddress,
    });
  },

  async listProducts(page: number = 0, pageSize: number = 20, callerAddress: string): Promise<any[]> {
    const simulated = await buildAndSimulateTransaction({
      method: "list_products",
      args: [page, pageSize],
      callerAddress,
    });

    if (SorobanRpc.isSimulationSuccess(simulated)) {
      return scValToNative(simulated.results?.[0]) || [];
    }
    throw new Error("Failed to list products");
  },

  async getProductCount(callerAddress: string): Promise<number> {
    const simulated = await buildAndSimulateTransaction({
      method: "get_product_count",
      args: [],
      callerAddress,
    });

    if (SorobanRpc.isSimulationSuccess(simulated)) {
      return scValToNative(simulated.results?.[0]) || 0;
    }
    throw new Error("Failed to get product count");
  },

  // ── Archival ──────────────────────────────────────────────────────────────

  async archiveTrackingEvent(
    productId: string,
    eventIndex: number,
    callerAddress: string
  ): Promise<string> {
    return buildSignAndSubmitTransaction({
      method: "archive_tracking_event",
      args: [productId, new Address(callerAddress), eventIndex],
      callerAddress,
    });
  },

  async listArchivedEvents(productId: string, callerAddress: string): Promise<any[]> {
    const simulated = await buildAndSimulateTransaction({
      method: "list_archived_events",
      args: [productId],
      callerAddress,
    });

    if (SorobanRpc.isSimulationSuccess(simulated)) {
      return scValToNative(simulated.results?.[0]) || [];
    }
    throw new Error("Failed to list archived events");
  },

  // ── Certification Registry ────────────────────────────────────────────────

  async issueCertification(
    productId: string,
    certId: string,
    certType: string,
    reference: string,
    callerAddress: string
  ): Promise<string> {
    return buildSignAndSubmitTransaction({
      method: "issue_certification",
      args: [productId, new Address(callerAddress), certId, certType, reference],
      callerAddress,
    });
  },

  async revokeCertification(
    productId: string,
    certId: string,
    callerAddress: string
  ): Promise<string> {
    return buildSignAndSubmitTransaction({
      method: "revoke_certification",
      args: [productId, new Address(callerAddress), certId],
      callerAddress,
    });
  },

  async verifyCertification(
    productId: string,
    certId: string,
    callerAddress: string
  ): Promise<any> {
    const simulated = await buildAndSimulateTransaction({
      method: "verify_certification",
      args: [productId, certId],
      callerAddress,
    });

    if (SorobanRpc.isSimulationSuccess(simulated)) {
      return scValToNative(simulated.results?.[0]);
    }
    throw new Error("Certification verification failed");
  },

  async getCertifications(productId: string, callerAddress: string): Promise<any[]> {
    const simulated = await buildAndSimulateTransaction({
      method: "get_certifications",
      args: [productId],
      callerAddress,
    });

    if (SorobanRpc.isSimulationSuccess(simulated)) {
      return scValToNative(simulated.results?.[0]) || [];
    }
    throw new Error("Failed to get certifications");
  },

  // ── Emergency Stop ────────────────────────────────────────────────────────

  async isPaused(callerAddress: string): Promise<boolean> {
    const simulated = await buildAndSimulateTransaction({
      method: "is_paused",
      args: [],
      callerAddress,
    });
    if (SorobanRpc.isSimulationSuccess(simulated)) {
      return scValToNative(simulated.results?.[0]) ?? false;
    }
    return false;
  },

  async setPauseState(paused: boolean, callerAddress: string): Promise<string> {
    return buildSignAndSubmitTransaction({
      method: "set_pause_state",
      args: [new Address(callerAddress), paused],
      callerAddress,
    });
  },

  async initGuardian(guardian: string, callerAddress: string): Promise<string> {
    return buildSignAndSubmitTransaction({
      method: "init_guardian",
      args: [new Address(guardian)],
      callerAddress,
    });
  },

  async addGuardian(newGuardian: string, callerAddress: string): Promise<string> {
    return buildSignAndSubmitTransaction({
      method: "add_guardian",
      args: [new Address(callerAddress), new Address(newGuardian)],
      callerAddress,
    });
  },

  async getGuardians(callerAddress: string): Promise<string[]> {
    const simulated = await buildAndSimulateTransaction({
      method: "get_guardians",
      args: [],
      callerAddress,
    });
    if (SorobanRpc.isSimulationSuccess(simulated)) {
      return scValToNative(simulated.results?.[0]) || [];
    }
    return [];
  },
};
