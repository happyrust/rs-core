/**
 * E3D配置测试脚本
 * 用于验证配置解析和API连接功能
 */

import { loadE3dConfig, validateE3dConfig, buildWorldApiUrl } from '../utils/e3dConfigParser.js'
import { createE3dModelLoader } from '../utils/e3dModelLoader.js'

/**
 * 测试配置加载功能
 */
export async function testConfigLoading() {
  console.log('=== 开始测试E3D配置加载 ===')
  
  try {
    // 测试配置文件加载
    console.log('1. 测试配置文件加载...')
    const config = await loadE3dConfig()
    console.log('✅ 配置加载成功:', config)
    
    // 测试配置验证
    console.log('2. 测试配置验证...')
    const isValid = validateE3dConfig(config)
    console.log('✅ 配置验证结果:', isValid)
    
    // 测试URL构建
    console.log('3. 测试URL构建...')
    const worldApiUrl = buildWorldApiUrl(config)
    console.log('✅ 世界API URL:', worldApiUrl)
    
    return config
    
  } catch (error) {
    console.error('❌ 配置测试失败:', error)
    throw error
  }
}

/**
 * 测试API连接
 */
export async function testApiConnection(config) {
  console.log('=== 开始测试API连接 ===')
  
  try {
    const worldApiUrl = buildWorldApiUrl(config)
    console.log('1. 测试API连接:', worldApiUrl)
    
    const response = await fetch(worldApiUrl, {
      method: 'GET',
      headers: {
        'Accept': 'application/json'
      },
      timeout: 10000
    })
    
    console.log('✅ API响应状态:', response.status, response.statusText)
    
    if (response.ok) {
      const data = await response.json()
      console.log('✅ API响应数据:', data)
      return data
    } else {
      throw new Error(`API请求失败: ${response.status} ${response.statusText}`)
    }
    
  } catch (error) {
    console.error('❌ API连接测试失败:', error)
    throw error
  }
}

/**
 * 测试模型加载器
 */
export async function testModelLoader(config) {
  console.log('=== 开始测试模型加载器 ===')
  
  try {
    console.log('1. 创建模型加载器...')
    const modelLoader = createE3dModelLoader(config)
    console.log('✅ 模型加载器创建成功')
    
    console.log('2. 获取世界根节点...')
    const worldRoot = await modelLoader.getWorldRoot()
    console.log('✅ 世界根节点:', worldRoot)
    
    console.log('3. 获取缓存统计...')
    const cacheStats = modelLoader.getCacheStats()
    console.log('✅ 缓存统计:', cacheStats)
    
    return { modelLoader, worldRoot }
    
  } catch (error) {
    console.error('❌ 模型加载器测试失败:', error)
    throw error
  }
}

/**
 * 运行完整测试套件
 */
export async function runFullTest() {
  console.log('🚀 开始运行E3D完整测试套件')
  
  try {
    // 1. 测试配置加载
    const config = await testConfigLoading()
    
    // 2. 测试API连接
    const apiData = await testApiConnection(config)
    
    // 3. 测试模型加载器
    const { modelLoader, worldRoot } = await testModelLoader(config)
    
    console.log('🎉 所有测试通过！')
    
    return {
      config,
      apiData,
      modelLoader,
      worldRoot,
      success: true
    }
    
  } catch (error) {
    console.error('💥 测试失败:', error)
    return {
      error: error.message,
      success: false
    }
  }
}

/**
 * 在浏览器控制台中运行测试
 */
if (typeof window !== 'undefined') {
  // 将测试函数暴露到全局作用域
  window.E3D_TEST = {
    testConfigLoading,
    testApiConnection,
    testModelLoader,
    runFullTest
  }
  
  console.log('E3D测试工具已加载，使用以下命令进行测试:')
  console.log('- E3D_TEST.testConfigLoading() - 测试配置加载')
  console.log('- E3D_TEST.testApiConnection(config) - 测试API连接')
  console.log('- E3D_TEST.testModelLoader(config) - 测试模型加载器')
  console.log('- E3D_TEST.runFullTest() - 运行完整测试')
}
