// freq.c: time a fixed ALU loop after idle gaps (frequency ramp probe)
// usage: freq GAP_MS MODE   MODE 0 default, 1 QoS user-interactive, 2 time-constraint policy,
//        3 QoS + time constraint
#include <stdio.h>
#include <stdlib.h>
#include <time.h>
#include <unistd.h>
#include <pthread.h>
#include <sys/qos.h>
#include <mach/mach.h>
#include <mach/thread_policy.h>
#include <mach/mach_time.h>
static double now(void){struct timespec t;clock_gettime(CLOCK_MONOTONIC_RAW,&t);return t.tv_sec*1e3+t.tv_nsec*1e-6;}
static void rt(double period_ms,double comp_ms,double cons_ms){
  mach_timebase_info_data_t tb; mach_timebase_info(&tb);
  double ms2abs=((double)tb.denom/(double)tb.numer)*1e6;
  thread_time_constraint_policy_data_t p;
  p.period=(uint32_t)(period_ms*ms2abs); p.computation=(uint32_t)(comp_ms*ms2abs);
  p.constraint=(uint32_t)(cons_ms*ms2abs); p.preemptible=1;
  kern_return_t r=thread_policy_set(pthread_mach_thread_np(pthread_self()),THREAD_TIME_CONSTRAINT_POLICY,(thread_policy_t)&p,THREAD_TIME_CONSTRAINT_POLICY_COUNT);
  if(r) fprintf(stderr,"thread_policy_set %d\n",r);
}
int main(int argc,char**argv){
  int mode=argc>2?atoi(argv[2]):0;
  if(mode&1) pthread_set_qos_class_self_np(QOS_CLASS_USER_INTERACTIVE,0);
  if(mode&2) rt(atof(getenv("P")),atof(getenv("C")),atof(getenv("K")));
  int gap=atoi(argv[1]);
  volatile unsigned long sink=0; double all[64]; int na=0;
  for(int r=0;r<40;r++){
    usleep(gap*1000);
    if(mode&4) rt(atof(getenv("P")),atof(getenv("C")),atof(getenv("K")));
    double t0=now();
    unsigned long x=r+1;
    double marks[4]; int m=0;
    for(int k=0;k<4;k++){ for(long i=0;i<2000000;i++){x=x*6364136223846793005UL+1442695040888963407UL;} marks[m++]=now()-t0;}
    if(mode&4){thread_standard_policy_data_t sp={0}; thread_policy_set(pthread_mach_thread_np(pthread_self()),THREAD_STANDARD_POLICY,(thread_policy_t)&sp,THREAD_STANDARD_POLICY_COUNT);}
    sink+=x; if(r>=2) all[na++]=marks[3];
    if(r>=2 && getenv("V")) printf("gap %d mode %d: chunks %.2f %.2f %.2f %.2f ms\n",gap,mode,marks[0],marks[1]-marks[0],marks[2]-marks[1],marks[3]-marks[2]);
  }
  for(int i=0;i<na;i++)for(int j=i+1;j<na;j++)if(all[j]<all[i]){double t=all[i];all[i]=all[j];all[j]=t;} printf("gap %d mode %d: 8M-iter total p50 %.2f p90 %.2f ms (n %d)\n",gap,mode,all[na/2],all[na*9/10],na);
  return 0;
}
