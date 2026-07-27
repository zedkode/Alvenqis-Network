import bcrypt from 'bcryptjs'
import dotenv from 'dotenv'
import { PrismaClient } from '@prisma/client'

dotenv.config()

const prisma = new PrismaClient()

const defaultParams = [
  ['block_time_seconds', 60],
  ['max_supply', '60000000'],
  ['blocks_per_year', 525600],
  ['halving_interval', 1576800],
  ['current_reward', '19.02587519'],
  ['difficulty_target', 'simulated'],
  ['network_mode', 'mainnet_candidate'],
  ['ticker', 'ALVE'],
]

const FORBIDDEN_DEFAULT_PASSWORDS = new Set([
  'ChangeMe123!',
  'changeme',
  'password',
  'admin',
  'admin123',
])

async function main() {
  const email = process.env.DEFAULT_ADMIN_EMAIL || 'admin@alvenqis.network'
  const password = process.env.DEFAULT_ADMIN_PASSWORD
  if (!password || password.trim().length < 12) {
    throw new Error(
      'DEFAULT_ADMIN_PASSWORD must be set to a strong password (min 12 chars). Refusing hardcoded seed defaults.',
    )
  }
  if (FORBIDDEN_DEFAULT_PASSWORDS.has(password) || FORBIDDEN_DEFAULT_PASSWORDS.has(password.trim())) {
    throw new Error('DEFAULT_ADMIN_PASSWORD uses a known weak/default value. Choose a unique strong password.')
  }
  const passwordHash = await bcrypt.hash(password, 12)

  const admin = await prisma.user.upsert({
    where: { email },
    update: { role: 'superadmin' },
    create: {
      email,
      passwordHash,
      role: 'superadmin',
    },
  })

  for (const [key, value] of defaultParams) {
    await prisma.networkParam.upsert({
      where: { key },
      update: { value, updatedBy: admin.id },
      create: { key, value, updatedBy: admin.id },
    })
  }

  await prisma.auditLog.create({
    data: {
      userId: admin.id,
      action: 'seed.initialized',
      entity: 'system',
      entityId: 'phase-1',
      diffJson: { defaultParams: defaultParams.map(([key]) => key) },
    },
  })

  console.log(`Seed complete. Superadmin: ${email}`)
}

main()
  .catch((error) => {
    console.error(error)
    process.exit(1)
  })
  .finally(async () => {
    await prisma.$disconnect()
  })
