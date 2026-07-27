import dotenv from 'dotenv'
import { z } from 'zod'

dotenv.config()

const envSchema = z.object({
  DATABASE_URL: z.string().min(1),
  JWT_SECRET: z.string().min(16),
  JWT_REFRESH_SECRET: z.string().min(16),
  PORT: z.coerce.number().default(4000),
  CORS_ORIGIN: z.string().default('http://localhost:5173'),
  RATE_LIMIT_WINDOW_MS: z.coerce.number().default(15 * 60 * 1000),
  RATE_LIMIT_MAX: z.coerce.number().default(300),
  NODE_ENV: z.enum(['development', 'test', 'production']).default('development'),
  ALVENQIS_RPC_URL: z.string().url().default('https://rpcnode.dohotstudio.com'),
})

const parsedEnv = envSchema.parse(process.env)

export const env = {
  ...parsedEnv,
  CORS_ORIGINS: parsedEnv.CORS_ORIGIN.split(',').map((origin) => origin.trim()).filter(Boolean),
}
