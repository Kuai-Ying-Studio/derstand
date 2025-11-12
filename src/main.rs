use std::io::{self, Read, Write};
use std::path::Path;
use std::process;
use std::time::Instant;

// 内存大小常量 - 优化的内存使用
const MEMORY_SIZE: usize = 30000;

/// Derstand指令枚举 - 12个基本指令
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Instruction {
    Right,    // > 指针右移
    Left,     // < 指针左移  
    Increment, // + 值加1
    Decrement, // - 值减1
    Output,   // . 输出
    Input,    // , 输入
    JumpIfZero, // [ 跳到对应的]
    JumpIfNotZero, // ] 跳回对应的[
    Zero,     // # 快速清零
    Copy,     // $ 复制到下一单元格
    MoveHigh, // % 移动到高端边界
    MoveLow,  // & 移动到低端边界
}

/// 高效的跳转表结构 - 避免运行时计算跳转位置
#[derive(Debug, Clone)]
struct JumpTable {
    // 跳转到对应右括号的位置: '[' -> 位置
    to_close: Vec<usize>,
    // 跳转到对应左括号的位置: ']' -> 位置  
    to_open: Vec<usize>,
}

/// Derstand解释器 - 优化版本
pub struct DerstandInterpreter {
    memory: [u8; MEMORY_SIZE], // 零拷贝内存访问
    pointer: usize,
    instructions: Vec<Instruction>,
    jump_table: JumpTable, // 优化后的跳转表
    input_buffer: Vec<u8>,
    output_buffer: Vec<u8>,
}

impl DerstandInterpreter {
    /// 创建新的解释器实例 - 最小化内存分配
    pub fn new() -> Self {
        DerstandInterpreter {
            memory: [0; MEMORY_SIZE],
            pointer: 0,
            instructions: Vec::with_capacity(1024), // 预分配指令空间
            jump_table: JumpTable {
                to_close: Vec::with_capacity(512),
                to_open: Vec::with_capacity(512),
            },
            input_buffer: Vec::with_capacity(256),
            output_buffer: Vec::with_capacity(256),
        }
    }

    /// 编译源代码 - 优化版本
    pub fn compile(&mut self, source: &str) -> Result<(), String> {
        self.instructions.clear();
        self.jump_table.to_close.clear();
        self.jump_table.to_open.clear();
        self.instructions.reserve(source.len()); // 预分配空间
        
        // 第一遍：解析指令
        let mut bracket_stack = Vec::with_capacity(128);
        
        for (pos, c) in source.chars().enumerate() {
            match c {
                '>' => self.instructions.push(Instruction::Right),
                '<' => self.instructions.push(Instruction::Left),
                '+' => self.instructions.push(Instruction::Increment),
                '-' => self.instructions.push(Instruction::Decrement),
                '.' => self.instructions.push(Instruction::Output),
                ',' => self.instructions.push(Instruction::Input),
                '[' => {
                    self.instructions.push(Instruction::JumpIfZero);
                    bracket_stack.push(pos);
                },
                ']' => {
                    self.instructions.push(Instruction::JumpIfNotZero);
                    if let Some(open_pos) = bracket_stack.pop() {
                        // 确保跳转表足够大
                        while self.jump_table.to_close.len() <= open_pos {
                            self.jump_table.to_close.push(0);
                        }
                        while self.jump_table.to_open.len() <= pos {
                            self.jump_table.to_open.push(0);
                        }
                        self.jump_table.to_close[open_pos] = pos;
                        self.jump_table.to_open[pos] = open_pos;
                    } else {
                        return Err(format!("Unmatched closing bracket at position {}", pos));
                    }
                },
                '#' => self.instructions.push(Instruction::Zero),
                '$' => self.instructions.push(Instruction::Copy),
                '%' => self.instructions.push(Instruction::MoveHigh),
                '&' => self.instructions.push(Instruction::MoveLow),
                _ => { /* 忽略非指令字符 */ },
            }
        }
        
        // 检查未匹配的左括号
        if !bracket_stack.is_empty() {
            return Err(format!("Unmatched opening bracket at position {}", bracket_stack[0]));
        }
        
        Ok(())
    }

    /// 执行编译后的指令 - 高度优化的执行循环
    pub fn execute(&mut self) -> Result<String, String> {
        self.pointer = 0;
        self.output_buffer.clear();
        
        let mut pc = 0; // 程序计数器
        
        // 优化的执行循环
        while pc < self.instructions.len() {
            match self.instructions[pc] {
                Instruction::Right => {
                    // 优化的边界检查
                    if self.pointer < MEMORY_SIZE - 1 {
                        self.pointer += 1;
                    }
                    pc += 1;
                },
                Instruction::Left => {
                    // 优化的边界检查
                    if self.pointer > 0 {
                        self.pointer -= 1;
                    }
                    pc += 1;
                },
                Instruction::Increment => {
                    // 无分支的单字节操作
                    self.memory[self.pointer] = self.memory[self.pointer].wrapping_add(1);
                    pc += 1;
                },
                Instruction::Decrement => {
                    // 无分支的单字节操作
                    self.memory[self.pointer] = self.memory[self.pointer].wrapping_sub(1);
                    pc += 1;
                },
                Instruction::Output => {
                    // 批量输出优化
                    self.output_buffer.push(self.memory[self.pointer]);
                    pc += 1;
                },
                Instruction::Input => {
                    // 处理输入
                    let byte = match self.input_buffer.pop() {
                        Some(b) => b,
                        None => {
                            let mut input = [0u8];
                            match io::stdin().read(&mut input) {
                                Ok(1) => input[0],
                                Ok(_) => 0,
                                Err(e) => return Err(format!("Input error: {}", e)),
                            }
                        },
                    };
                    self.memory[self.pointer] = byte;
                    pc += 1;
                },
                Instruction::JumpIfZero => {
                    // 高效跳转 - 使用预计算的跳转表
                    if self.memory[self.pointer] == 0 {
                        if pc < self.jump_table.to_close.len() {
                            pc = self.jump_table.to_close[pc] + 1;
                        } else {
                            return Err("Jump table out of bounds".to_string());
                        }
                    } else {
                        pc += 1;
                    }
                },
                Instruction::JumpIfNotZero => {
                    // 高效跳转 - 使用预计算的跳转表
                    if self.memory[self.pointer] != 0 {
                        if pc < self.jump_table.to_open.len() {
                            pc = self.jump_table.to_open[pc] + 1;
                        } else {
                            return Err("Jump table out of bounds".to_string());
                        }
                    } else {
                        pc += 1;
                    }
                },
                Instruction::Zero => {
                    // 快速清零 - 比多次减操作更高效
                    self.memory[self.pointer] = 0;
                    pc += 1;
                },
                Instruction::Copy => {
                    // 复制当前值到下一单元格
                    if self.pointer < MEMORY_SIZE - 1 {
                        self.memory[self.pointer + 1] = self.memory[self.pointer];
                    }
                    pc += 1;
                },
                Instruction::MoveHigh => {
                    // 移动到高端边界
                    self.pointer = MEMORY_SIZE - 1;
                    pc += 1;
                },
                Instruction::MoveLow => {
                    // 移动到低端边界
                    self.pointer = 0;
                    pc += 1;
                },
            }
        }
        
        // 将输出缓冲区转换为字符串
        Ok(String::from_utf8_lossy(&self.output_buffer).to_string())
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut interpreter = DerstandInterpreter::new();
    
    if args.len() > 1 {
        // 文件模式
        let file_path = &args[1];
        if !Path::new(file_path).exists() {
            eprintln!("File not found: {}", file_path);
            process::exit(1);
        }
        
        let source = std::fs::read_to_string(file_path)
            .unwrap_or_else(|e| {
                eprintln!("Error reading file: {}", e);
                process::exit(1);
            });
        
        // 编译和执行
        match interpreter.compile(&source) {
            Ok(_) => {
                // 开始计时
                let start_time = Instant::now();
                
                match interpreter.execute() {
                    Ok(output) => {
                        // 结束计时并计算时间
                        let elapsed = start_time.elapsed();
                        print!("{}", output);
                        println!("\nExecution time: {}.{} ms", elapsed.as_millis(), elapsed.subsec_micros() / 1000);
                    },
                    Err(e) => {
                        eprintln!("Execution error: {}", e);
                        process::exit(1);
                    },
                }
            },
            Err(e) => {
                eprintln!("Compilation error: {}", e);
                process::exit(1);
            },
        }
    } else {
        // 交互式模式
        println!("Derstand Interpreter v0.1.0");
        println!("Instructions: > < + - . , [ ] # $ % &");
        println!("Type 'quit' to exit.");
        
        loop {
            print!("\n> ");
            io::stdout().flush().unwrap();
            
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            
            let input = input.trim();
            if input == "quit" || input == "exit" {
                break;
            }
            if input.is_empty() {
                continue;
            }
            
            // 编译和执行
            match interpreter.compile(input) {
                Ok(_) => {
                    // 开始计时
                    let start_time = Instant::now();
                    
                    match interpreter.execute() {
                        Ok(output) => {
                            // 结束计时并计算时间
                            let elapsed = start_time.elapsed();
                            if !output.is_empty() {
                                println!("Output: {}", output);
                            } else {
                                println!("(no output)");
                            }
                            println!("Execution time: {}.{} ms", elapsed.as_millis(), elapsed.subsec_micros() / 1000);
                        },
                        Err(e) => println!("Execution error: {}", e),
                    }
                },
                Err(e) => println!("Compilation error: {}", e),
            }
        }
    }
}