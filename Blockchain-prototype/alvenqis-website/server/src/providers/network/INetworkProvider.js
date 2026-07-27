export class INetworkProvider {
  async getBlocks() {
    throw new Error('INetworkProvider.getBlocks must be implemented')
  }

  async getBlockByHeight() {
    throw new Error('INetworkProvider.getBlockByHeight must be implemented')
  }

  async getStats() {
    throw new Error('INetworkProvider.getStats must be implemented')
  }
}
