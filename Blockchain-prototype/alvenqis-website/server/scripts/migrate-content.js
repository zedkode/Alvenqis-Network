import { PrismaClient } from '@prisma/client'
import { fileURLToPath, pathToFileURL } from 'node:url'
import path from 'node:path'
import dotenv from 'dotenv'

dotenv.config()

const prisma = new PrismaClient()
const __dirname = path.dirname(fileURLToPath(import.meta.url))
const contentPath = path.resolve(__dirname, '..', '..', 'src', 'data', 'content.js')

const pageSections = {
  global: ['navItems', 'networkStats', 'standards', 'openDecisions'],
  home: ['confirmationModel', 'ecosystemProducts', 'offChainItems', 'onChainItems', 'productLayers', 'roadmap', 'standards'],
  core: ['chainFacts', 'confirmationModel', 'coreModules', 'openDecisions'],
  mining: ['miningModules', 'networkStats'],
  wallet: ['walletFeatures'],
  explorer: ['explorerFeatures'],
  developers: ['developerStack', 'standards'],
  network: ['productLayers'],
  protocol: ['networkStats', 'openDecisions'],
  passport: ['passportUseCases'],
  ecosystem: ['ecosystemProducts', 'standards'],
  whitepaper: ['whitepaperSections'],
  roadmap: ['roadmap'],
  docs: ['docsCards'],
  status: ['openDecisions'],
  tokenomics: ['tokenomicsRows'],
  faq: ['faqItems'],
}

const roadmapStatusMap = {
  active: 'active',
  next: 'next',
  planned: 'planned',
  research: 'research',
  future: 'future',
  completed: 'completed',
}

function normalizeStatus(status) {
  return roadmapStatusMap[String(status).toLowerCase()] || 'planned'
}

async function upsertContentBlock({ pageSlug, sectionKey, contentJson, lang = 'en' }) {
  return prisma.contentBlock.upsert({
    where: {
      pageSlug_sectionKey_lang: {
        pageSlug,
        sectionKey,
        lang,
      },
    },
    update: { contentJson },
    create: {
      pageSlug,
      sectionKey,
      contentJson,
      lang,
    },
  })
}

async function main() {
  const content = await import(pathToFileURL(contentPath).href)
  let contentBlockCount = 0

  for (const [pageSlug, sectionKeys] of Object.entries(pageSections)) {
    for (const sectionKey of sectionKeys) {
      if (content[sectionKey] === undefined) continue

      await upsertContentBlock({
        pageSlug,
        sectionKey,
        contentJson: content[sectionKey],
        lang: 'en',
      })
      contentBlockCount += 1
    }
  }

  if (Array.isArray(content.roadmap)) {
    for (const [index, item] of content.roadmap.entries()) {
      const [phase, title, status, description] = item

      await prisma.roadmapItem.upsert({
        where: { id: `seed-roadmap-${index}` },
        update: {
          phase,
          title,
          description,
          status: normalizeStatus(status),
          order: index,
        },
        create: {
          id: `seed-roadmap-${index}`,
          phase,
          title,
          description,
          status: normalizeStatus(status),
          order: index,
        },
      })
    }
  }

  if (Array.isArray(content.faqItems)) {
    for (const [index, item] of content.faqItems.entries()) {
      const [question, answer] = item

      await prisma.faqItem.upsert({
        where: { id: `seed-faq-en-${index}` },
        update: {
          question,
          contentJson: { answer },
          order: index,
          lang: 'en',
        },
        create: {
          id: `seed-faq-en-${index}`,
          question,
          contentJson: { answer },
          order: index,
          lang: 'en',
        },
      })
    }
  }

  await prisma.auditLog.create({
    data: {
      action: 'content.migrated',
      entity: 'content_blocks',
      entityId: 'src/data/content.js',
      diffJson: {
        source: 'src/data/content.js',
        contentBlocks: contentBlockCount,
        roadmapItems: Array.isArray(content.roadmap) ? content.roadmap.length : 0,
        faqItems: Array.isArray(content.faqItems) ? content.faqItems.length : 0,
      },
    },
  })

  console.log(`Content migration complete. Content blocks: ${contentBlockCount}`)
}

main()
  .catch((error) => {
    console.error(error)
    process.exit(1)
  })
  .finally(async () => {
    await prisma.$disconnect()
  })
