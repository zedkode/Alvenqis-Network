import app from './app.js'
import { env } from './config/env.js'
import { prisma } from './prisma/client.js'

const server = app.listen(env.PORT, () => {
  console.log(`Alvenqis server listening on http://localhost:${env.PORT}`)
})

async function shutdown() {
  server.close(async () => {
    await prisma.$disconnect()
    process.exit(0)
  })
}

process.on('SIGINT', shutdown)
process.on('SIGTERM', shutdown)
