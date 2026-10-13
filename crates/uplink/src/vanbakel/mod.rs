//! Port of `lib/vanbakel`: the CPU-share task scheduler behind Uplink's running software.
//!
//! Tasks are `Box<dyn UplinkTask>` held in [`TaskWrapper`]s. Like screens, a task is taken out of
//! its wrapper while one of its methods runs; `remove_task` on a running task is completed as soon
//! as the method returns.

pub mod interface;

use crate::ctx::Ctx;
use crate::interface::taskmanager::uplinktask::UplinkTask;

pub struct TaskWrapper {
    pub pid: i32,
    pub name: String,
    pub task: Option<Box<dyn UplinkTask>>,
    pub priority: f64,
    pub progress: f64,
    /// Set when `remove_task` was called while the task was running.
    remove_pending: bool,
}

impl TaskWrapper {
    pub fn debug_print(&self) {
        println!("TaskWrapper pid:{} name:{} priority:{} progress:{}", self.pid, self.name, self.priority, self.progress);
    }
}

#[derive(Default)]
pub struct Vanbakel {
    pub tasklist: Vec<TaskWrapper>,
    nextpid: i32,
    /// Interface state (see `interface.rs`).
    pub xpos: i32,
    pub ypos: i32,
    pub numregisteredbuttons: i32,
}

impl Vanbakel {
    /// `SvbReset`
    pub fn reset(g: &mut Ctx) {
        g.svb.tasklist.clear();
        g.svb.nextpid = 0;
        interface::initialise_interface(g);
    }

    /// `SvbRegisterTask(name, task)`: returns the new PID.
    pub fn register_task(g: &mut Ctx, name: &str, mut task: Box<dyn UplinkTask>) -> i32 {
        g.svb.nextpid += 1;
        let pid = g.svb.nextpid;
        task.base_mut().pid = pid;
        g.svb.tasklist.push(TaskWrapper {
            pid,
            name: name.to_string(),
            task: Some(task),
            priority: 0.0,
            progress: 0.0,
            remove_pending: false,
        });
        g.svb.calculate_priorities_add();
        Self::update_interface(g);
        Self::with_task(g, pid, |t, g| t.initialise(g));
        if Self::is_interface_visible(g) {
            Self::with_task(g, pid, |t, g| t.create_interface(g));
        }
        pid
    }

    /// Run `f` on the task with this PID (taken out of the list while `f` runs).
    /// Returns `None` if there is no such task or it is already running (re-entrant call).
    pub fn with_task<R>(g: &mut Ctx, pid: i32, f: impl FnOnce(&mut dyn UplinkTask, &mut Ctx) -> R) -> Option<R> {
        let idx = g.svb.lookup_index(pid);
        if idx < 0 {
            return None;
        }
        let mut task = g.svb.tasklist[idx as usize].task.take()?;
        let r = f(task.as_mut(), g);
        let idx = g.svb.lookup_index(pid);
        if idx >= 0 {
            let remove = g.svb.tasklist[idx as usize].remove_pending;
            g.svb.tasklist[idx as usize].task = Some(task);
            if remove {
                Self::remove_task(g, pid);
            }
        }
        g.run_deferred();
        Some(r)
    }

    /// `SvbRemoveTask(pid)`: calls the task's `remove_interface`, then drops it.
    pub fn remove_task(g: &mut Ctx, pid: i32) {
        let idx = g.svb.lookup_index(pid);
        if idx >= 0 {
            let i = idx as usize;
            if g.svb.tasklist[i].task.is_none() {
                // The task is running right now; finish when it returns.
                g.svb.tasklist[i].remove_pending = true;
                return;
            }
            let mut task = g.svb.tasklist[i].task.take().expect("task present");
            task.remove_interface(g);
            let idx = g.svb.lookup_index(pid);
            if idx >= 0 {
                let removed = g.svb.tasklist.remove(idx as usize);
                g.svb.calculate_priorities_remove(removed.priority);
            }
        }
        Self::update_interface(g);
    }

    /// `SvbNumTasks`
    pub fn num_tasks(&self) -> i32 {
        self.tasklist.len() as i32
    }

    /// `SvbLookupIndex(pid)`: -1 if not found.
    pub fn lookup_index(&self, pid: i32) -> i32 {
        self.tasklist.iter().position(|t| t.pid == pid).map(|i| i as i32).unwrap_or(-1)
    }

    /// `SvbGetTask(pid)` downcast to a concrete type. `None` if missing, other type, or running.
    pub fn get_task<T: UplinkTask>(&self, pid: i32) -> Option<&T> {
        self.tasklist.iter().find(|t| t.pid == pid)?.task.as_ref()?.as_any().downcast_ref::<T>()
    }

    pub fn get_task_mut<T: UplinkTask>(&mut self, pid: i32) -> Option<&mut T> {
        self.tasklist.iter_mut().find(|t| t.pid == pid)?.task.as_mut()?.as_any_mut().downcast_mut::<T>()
    }

    /// `SvbGetTask(pid)` as a trait object.
    pub fn get_task_dyn(&self, pid: i32) -> Option<&dyn UplinkTask> {
        self.tasklist.iter().find(|t| t.pid == pid)?.task.as_deref()
    }

    pub fn get_task_dyn_mut(&mut self, pid: i32) -> Option<&mut (dyn UplinkTask + 'static)> {
        self.tasklist.iter_mut().find(|t| t.pid == pid)?.task.as_deref_mut()
    }

    /// True if a task with this PID exists (even if it is running right now).
    pub fn task_exists(&self, pid: i32) -> bool {
        self.lookup_index(pid) >= 0
    }

    /// `SvbGetTask(name)`: PID of the first task with this name, or -1.
    pub fn get_task_by_name(&self, name: &str) -> i32 {
        self.tasklist.iter().find(|t| t.name == name).map(|t| t.pid).unwrap_or(-1)
    }

    /// `SvbGetTaskWrapper(pid)`
    pub fn get_task_wrapper(&self, pid: i32) -> Option<&TaskWrapper> {
        self.tasklist.iter().find(|t| t.pid == pid)
    }

    pub fn get_task_wrapper_at_index(&self, index: i32) -> Option<&TaskWrapper> {
        if index < 0 {
            return None;
        }
        self.tasklist.get(index as usize)
    }

    /// PIDs of all tasks, in list order.
    pub fn pids(&self) -> Vec<i32> {
        self.tasklist.iter().map(|t| t.pid).collect()
    }

    fn calculate_priorities_add(&mut self) {
        let n = self.tasklist.len();
        let newpriority = 1.0 / n as f64;
        for i in 0..n - 1 {
            let p = self.tasklist[i].priority;
            self.change_priority(i as i32, p * -newpriority, true);
        }
        self.change_priority(n as i32 - 1, newpriority, true);
        self.compensate_priorities();
    }

    pub fn calculate_priorities_remove(&mut self, removedpriority: f64) {
        let n = self.tasklist.len();
        if n == 0 {
            return;
        }
        let dp = (n as f64 + 1.0) / n as f64;
        for i in 0..n {
            let p = self.tasklist[i].priority;
            self.change_priority(i as i32, p * removedpriority * dp, true);
        }
        self.compensate_priorities();
    }

    /// `SvbChangePriority(index, amount, locked)`
    pub fn change_priority(&mut self, index: i32, amount: f64, locked: bool) {
        if index < 0 || index as usize >= self.tasklist.len() {
            return;
        }
        let i = index as usize;
        let tw = &mut self.tasklist[i];
        let actualamount = if tw.priority + amount > 1.0 {
            let a = 1.0 - tw.priority;
            tw.priority = 1.0;
            a
        } else if tw.priority + amount < 0.0 {
            let a = -tw.priority;
            tw.priority = 0.0;
            a
        } else {
            tw.priority += amount;
            amount
        };
        if !locked {
            let n = self.tasklist.len();
            if n > 1 {
                let dp = n as f64 / (n as f64 - 1.0);
                for j in 0..n {
                    if j != i {
                        let p = self.tasklist[j].priority;
                        self.change_priority(j as i32, p * actualamount * -dp, true);
                    }
                }
            }
        }
    }

    fn compensate_priorities(&mut self) {
        let total: f64 = self.tasklist.iter().map(|t| t.priority).sum();
        let error = 1.0 - total;
        if error > 0.05 {
            for i in 0..self.tasklist.len() {
                let p = self.tasklist[i].priority;
                self.change_priority(i as i32, p * error, true);
            }
        }
    }

    /// `SvbCycle(n)`: share `n` ticks between the tasks by priority.
    pub fn cycle(g: &mut Ctx, n: i32) {
        for pid in g.svb.pids() {
            let idx = g.svb.lookup_index(pid);
            if idx < 0 {
                continue;
            }
            let tw = &mut g.svb.tasklist[idx as usize];
            tw.progress += n as f64 * tw.priority;
            if tw.progress > 1.0 {
                let numticks = 1 + (tw.progress - 1.0) as i32;
                tw.progress -= numticks as f64;
                Self::with_task(g, pid, |t, g| t.tick(g, numticks));
            }
        }
    }

    /// `SvbShowTask(pid)`
    pub fn show_task(g: &mut Ctx, pid: i32) {
        Self::with_task(g, pid, |t, g| t.create_interface(g));
    }

    /// `SvbHideTask(pid)`
    pub fn hide_task(g: &mut Ctx, pid: i32) {
        Self::with_task(g, pid, |t, g| t.remove_interface(g));
    }

    /// `SvbShowAllTasks`
    pub fn show_all_tasks(g: &mut Ctx) {
        for pid in g.svb.pids() {
            Self::with_task(g, pid, |t, g| t.show_interface(g));
        }
    }

    /// `SvbCreateInterface(x, y)`
    pub fn create_interface(g: &mut Ctx, x: i32, y: i32) {
        interface::create_interface(g, x, y);
    }

    /// `SvbRemoveInterface`
    pub fn remove_interface(g: &mut Ctx) {
        interface::remove_interface(g);
    }

    /// `SvbUpdateInterface`
    pub fn update_interface(g: &mut Ctx) {
        interface::update_interface(g);
    }

    /// `SvbIsInterfaceVisible`
    pub fn is_interface_visible(g: &Ctx) -> bool {
        g.ecl.get_button("svb_scalebar").is_some()
    }

    pub fn debug_print(&self) {
        println!("============== V A N  B A K E L ============================");
        println!("numregisteredbuttons:{}, xpos:{}, ypos:{}, nextpid:{}", self.numregisteredbuttons, self.xpos, self.ypos, self.nextpid);
        for t in &self.tasklist {
            t.debug_print();
        }
        println!("============== E N D  O F  V A N  B A K E L ================");
    }
}
