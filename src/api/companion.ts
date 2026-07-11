import { invoke } from '@tauri-apps/api/core'

export interface RelationshipState {
  personaId: string
  intimacy: number
  trust: number
  moodTowardUser: number
  daysKnown: number
  conversationCount: number
  nickname: string | null
  firstMetAt: string
  lastInteractionAt: string
  loveLanguage: string
  responseStyle: 'Polite' | 'Friendly' | 'Intimate' | 'Passionate'
}

export interface Milestone {
  id: string
  personaId: string
  typ: string
  title: string
  description: string
  icon: string
  achievedAt: string
}

export interface CoreMemory {
  personaId: string
  userProfile: {
    name: string | null
    nickname: string | null
    birthday: string | null
    age: number | null
    gender: string | null
    occupation: string | null
    city: string | null
    education: string | null
    mbti: string | null
  }
  preferences: {
    favoriteFoods: string[]
    favoriteMusic: string[]
    favoriteMovies: string[]
    favoriteGames: string[]
    hobbies: string[]
    dislikes: string[]
    favoriteColor: string | null
    routines: string[]
    sleepTime: string | null
    wakeTime: string | null
  }
  relationships: Array<{ name: string; relation: string; notes?: string }>
  pets: Array<{ name: string; species: string; notes?: string }>
  keyEvents: Array<{ date: string; title: string; description: string; emotion?: string }>
  sharedMemories: {
    ourSongs: string[]
    ourPlaces: string[]
    insideJokes: string[]
    promises: string[]
    specialDates: Array<{ date: string; label: string; note?: string }>
  }
  updatedAt: string
}

export interface RelationshipInfo {
  state: RelationshipState
  milestones: Milestone[]
  coreMemory: CoreMemory
}

export interface ProactiveMessage {
  id: string
  personaId: string
  personaName: string
  triggerType: string
  content: string
  createdAt: string
  delivered: boolean
}

export async function getRelationship(personaId: string): Promise<RelationshipInfo> {
  return invoke('get_relationship', { personaId })
}

export async function getAllRelationships(): Promise<RelationshipState[]> {
  return invoke('get_all_relationships')
}

export async function setNickname(personaId: string, nickname: string): Promise<RelationshipState> {
  return invoke('set_nickname', { personaId, nickname })
}

export async function resetRelationship(personaId: string): Promise<void> {
  return invoke('reset_relationship', { personaId })
}

export async function updateCoreMemory(personaId: string, memory: CoreMemory): Promise<void> {
  return invoke('update_core_memory', { personaId, memory })
}
